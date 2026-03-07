# TTstack Compatibility Matrix

This document describes the host OS and guest VM/container compatibility
for TTstack, including platforms that have been tested in practice and
those planned for support.

---

## Host OS Compatibility

TTstack agents and the controller can run on the following host
operating systems. The table below distinguishes between platforms
that have been **tested** (with automated or manual verification) and
those that are **planned** (expected to work based on architecture but
not yet verified in CI or lab environments).

| Host OS | Version | Status | Storage Backends | Engines | Notes |
|---------|---------|--------|-----------------|---------|-------|
| Debian | 13 (Trixie) | **Tested** | raw, ZFS, Btrfs | QEMU/KVM, Firecracker, Docker | Primary development platform |
| Alpine Linux | 3.23 | **Tested** | raw | QEMU/KVM, Docker | Requires `iproute2`, `nftables`, `socat`, `qemu-system-x86_64`; BusyBox `ip` is not sufficient |
| Ubuntu | 24.04 LTS | Planned | raw, ZFS, Btrfs | QEMU/KVM, Firecracker, Docker | Debian-derivative; expected full compatibility |
| Rocky Linux | 10.x | Planned | raw, ZFS, Btrfs | QEMU/KVM, Firecracker, Docker/Podman | RHEL-derivative; nftables is the default firewall backend |
| Gentoo | 23.0 | Planned | raw, ZFS, Btrfs | QEMU/KVM, Firecracker, Docker | Requires manual package installation |
| FreeBSD | 14.3 | **Tested** | raw, ZFS | Bhyve, Jail | Uses PF instead of nftables; clang from base system |

### Host OS Requirements

All Linux hosts require:

- **Kernel**: 5.10+ (for KVM, nftables, and namespace support)
- **nftables**: For NAT and port forwarding (replaces legacy iptables)
- **iproute2**: Full `ip` command (BusyBox `ip` lacks `tuntap` support)
- **socat**: For QEMU monitor socket communication
- **bridge-utils** or iproute2 bridge support
- **SQLite 3**: Linked at build time via `rusqlite` (bundled by default)

Engine-specific requirements:

| Engine | Required Packages | Kernel Features |
|--------|------------------|-----------------|
| QEMU/KVM | `qemu-system-x86_64`, `socat` | `/dev/kvm` (hardware virtualization) |
| Firecracker | `firecracker` binary | `/dev/kvm`, `tun` module |
| Docker | `docker` or `podman` | cgroups v2, overlayfs |

FreeBSD hosts require:

- **PF**: Packet filter for NAT (configured automatically)
- **Bhyve**: Native hypervisor (FreeBSD 10+)
- **Jail**: Native container isolation

### Storage Backend Requirements

| Backend | Host Requirements | Performance | Notes |
|---------|------------------|-------------|-------|
| raw | Any filesystem | Baseline | Uses `cp --reflink=auto` for CoW on supported FS |
| ZFS | ZFS pool mounted | Fast (instant clone) | Requires `zfs` CLI; images stored as datasets |
| Btrfs | Btrfs filesystem mounted | Fast (snapshot clone) | Requires `btrfs` CLI; images stored as subvolumes |

### Tested Host Configurations

The following configurations have been verified in practice:

#### Debian 13 (Trixie) — x86_64

- **Kernel**: 6.12.73+deb13
- **CPU**: 64 cores, **RAM**: 125 GiB
- **QEMU**: 10.0.7 with KVM acceleration
- **Firecracker**: Tested with fc-alpine image
- **Docker**: 26.1.5
- **Storage**: raw (default), ZFS (pool `ttpool`), Btrfs (`/mnt/btrfs-tt`)
- **Networking**: nftables with tt-nat table, bridge `tt0`

**Test results**:
- All three storage backends (raw, ZFS, Btrfs): VM create/destroy with proper clone/snapshot lifecycle
- QEMU engine: Full lifecycle (create → run → pause → resume → destroy)
- Firecracker engine: Full lifecycle (create → run → pause → destroy)
- Docker engine: Full lifecycle (create → run → stop → start → destroy) with native port mapping
- Concurrent environment creation (5 parallel): No conflicts
- Agent crash recovery: VM state and host_id persist across forced restarts
- Multi-host scheduling: VMs distributed across hosts when single host is insufficient

#### Alpine Linux 3.23 — x86_64

- **Kernel**: 6.18.9-0-lts
- **CPU**: 128 cores, **RAM**: 252 GiB
- **QEMU**: 10.1.3 with KVM acceleration
- **Docker**: 29.1.3
- **Storage**: raw
- **Networking**: nftables + iproute2 (required; BusyBox `ip` is insufficient)

**Required packages** (beyond base install):
```
apk add qemu-system-x86_64 qemu-img docker socat nftables iproute2 curl
modprobe tun  # if /dev/net/tun is missing
```

**Test results**:
- QEMU engine: Full lifecycle (create → run → pause → resume → destroy)
- Multi-host: Successfully participates as remote agent in distributed fleet
- Cross-host scheduling: VMs correctly placed and managed from central controller

#### FreeBSD 14.3-RELEASE — x86_64

- **Kernel**: 14.3-RELEASE GENERIC
- **Clang**: 19.1.7 (from base system)
- **Rust**: 1.92.0 (from pkg)
- **Storage**: raw
- **Engines detected**: bhyve, jail

**Test results**:
- All 50 unit tests pass natively on FreeBSD
- Agent: Starts successfully, bridge `tt0` created via `ifconfig bridge create name tt0`
- CLI: Connects to remote controller, displays fleet status and host list
- Controller: Builds and runs on FreeBSD (cross-platform component)
- Build: Compiles all three binaries (tt, tt-agent, tt-ctl) without modification

---

## Guest VM / Container Compatibility

### Container Engines (Docker / Podman)

TTstack delegates container management entirely to Docker or Podman.
Any container image that runs on the host's container runtime is
supported. This includes the full OCI container ecosystem:

| Category | Examples | Notes |
|----------|----------|-------|
| Official base images | `alpine`, `debian`, `ubuntu`, `rockylinux`, `fedora` | All tags supported |
| Language runtimes | `python`, `node`, `golang`, `rust`, `ruby`, `openjdk` | |
| Databases | `postgres`, `mysql`, `redis`, `mongodb`, `mariadb` | |
| Web servers | `nginx`, `httpd`, `caddy`, `traefik` | |
| Application platforms | `wordpress`, `grafana`, `prometheus`, `gitlab` | |
| Custom images | Any `Dockerfile`-built or registry image | |

**Tested container images**:
- `alpine:3.21` (minimal 754 KB test image)
- Custom `tt-test-alpine` (init-based test container)

### VM Engines (QEMU/KVM, Firecracker, Bhyve)

VM engines boot full operating system images. The guest OS must be
compatible with the virtual hardware presented by each engine.

#### QEMU/KVM Guest Compatibility

QEMU provides full x86_64 hardware emulation with KVM acceleration.
Any operating system that supports the `virtio` device family is
recommended for best performance.

**Supported guest OS families** (server editions, releases within the
last 3 years):

| Guest OS | Versions | Disk Format | Notes |
|----------|----------|-------------|-------|
| Debian | 11 (Bullseye), 12 (Bookworm), 13 (Trixie) | qcow2 | Excellent virtio support |
| Ubuntu Server | 22.04 LTS, 24.04 LTS | qcow2 | Cloud images work directly |
| Alpine Linux | 3.18–3.23 | qcow2 | Minimal footprint, fast boot |
| Rocky Linux | 8.x, 9.x, 10.x | qcow2 | RHEL-compatible |
| Alma Linux | 8.x, 9.x | qcow2 | RHEL-compatible |
| Fedora Server | 39, 40, 41 | qcow2 | Cutting-edge kernel |
| openSUSE Leap | 15.5, 15.6 | qcow2 | Enterprise-grade |
| Arch Linux | Rolling | qcow2 | Latest packages |
| Gentoo | 23.0 | qcow2 | Source-based |
| FreeBSD | 13.x, 14.x | qcow2 | Full virtio support since 12.0 |
| Windows Server | 2019, 2022, 2025 | qcow2 | Requires virtio-win drivers |

**Tested guest images**:
- Minimal qcow2 test image (256 MiB, lifecycle verification)
- Firecracker Alpine rootfs (kernel + ext4 rootfs)

**Guest image requirements**:
- Format: qcow2 (recommended) or raw
- For directory-based storage (ZFS/Btrfs): place the qcow2 file inside the dataset/subvolume directory
- The engine automatically resolves `disk.qcow2` inside directories

#### Firecracker Guest Compatibility

Firecracker boots microVMs with a Linux kernel and root filesystem.
It does **not** support full BIOS/UEFI boot — the guest must be a
Linux kernel (`vmlinux`) paired with an ext4 root filesystem.

| Guest OS | Versions | Image Format | Notes |
|----------|----------|-------------|-------|
| Alpine Linux | 3.18–3.23 | vmlinux + rootfs.ext4 | Ideal for microVMs |
| Debian | 11–13 | vmlinux + rootfs.ext4 | Requires kernel extraction |
| Ubuntu | 22.04, 24.04 | vmlinux + rootfs.ext4 | Cloud kernel works |
| Amazon Linux | 2, 2023 | vmlinux + rootfs.ext4 | Native Firecracker support |

**Firecracker image structure**:
```
images/fc-alpine/
├── vmlinux         # Uncompressed Linux kernel
└── rootfs.ext4     # Root filesystem
```

**Tested**: Alpine Linux microVM image with custom init.

#### Bhyve Guest Compatibility (FreeBSD hosts only)

Bhyve is FreeBSD's native hypervisor. It supports:

| Guest OS | Versions | Notes |
|----------|----------|-------|
| FreeBSD | 12.x–14.x | Native guest support |
| Linux | Recent kernels (5.x+) | Requires UEFI boot or grub-bhyve |
| OpenBSD | 7.x | |
| Windows | 10, 11, Server 2019+ | Experimental |

**Tested**: Build and unit tests pass on FreeBSD 14.3. Agent starts
and reports bhyve/jail engines. CLI and controller work correctly.

---

## Network Compatibility

| Feature | Linux (nftables) | FreeBSD (PF) |
|---------|------------------|--------------|
| Bridge networking | `tt0` bridge (10.10.0.1/16) | `tt0` bridge |
| TAP devices | Per-VM, auto-named | Per-VM |
| NAT / masquerade | nftables `tt-nat` table | PF rules |
| Port forwarding (DNAT) | nftables prerouting chain | PF rdr rules |
| Outgoing traffic block | nftables forward chain + denylist set | PF block rules |
| Docker networking | Native Docker `-p` (no nftables) | Native Docker |

**Tested**: Full nftables networking on Debian 13 and Alpine 3.23,
including port forwarding, NAT, deny-outgoing, and Docker-native
port publishing. Bridge and TAP creation on FreeBSD 14.3.

---

## Build Toolchain

| Component | Minimum Version | Tested Version |
|-----------|----------------|----------------|
| Rust | 1.86 (edition 2024) | 1.92.0–1.93.1 |
| Cargo | Matching Rust | 1.92.0–1.93.1 |
| C compiler | GCC or Clang | GCC 14.2 (Debian), GCC 15.2 (Alpine), Clang 19.1 (FreeBSD) |
| SQLite | 3.x (bundled) | Bundled via `libsqlite3-sys` |
| OpenSSL | 1.1+ or 3.x | 3.5.x |

**Alpine build note**: Requires `openssl-dev` and `openssl-libs-static`
for static linking. For musl cross-compilation from Debian:
`OPENSSL_STATIC=1 OPENSSL_NO_VENDOR=0 cargo build --release --target x86_64-unknown-linux-musl --features reqwest/native-tls-vendored`

**FreeBSD build note**: Requires `rust`, `pkgconf`, and `openssl` from pkg.
FreeBSD 14.3 ships clang 19 in base — no additional compiler setup needed.

---

## Summary

| Component | Tested | Planned |
|-----------|--------|---------|
| Host OS | Debian 13, Alpine 3.23, FreeBSD 14.3 | Ubuntu 24.04, Rocky 10, Gentoo 23 |
| VM engines | QEMU/KVM, Firecracker, Docker (Linux); Bhyve, Jail (FreeBSD) | — |
| Storage backends | raw, ZFS, Btrfs | All verified on Debian 13 |
| Multi-host | 3-host fleet (Debian + Alpine + FreeBSD) | Up to 50 hosts |
| Guest OS (containers) | Alpine-based test images | Full OCI ecosystem |
| Guest OS (VMs) | Minimal test images | See guest compatibility tables above |
