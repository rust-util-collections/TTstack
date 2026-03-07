# Guest Image Guide

TTstack supports multiple VM engines, each requiring its own image format.
This guide covers how to create, manage, and deploy guest images for each engine.

## Quick Start

Use the built-in image recipes:

```bash
# List available recipes
tt image recipes

# Create all images for this platform
sudo tt image create all --image-dir /home/ttstack/images

# Create all Docker images
sudo tt image create all --engine docker

# Create a specific image
sudo tt image create fc-alpine --image-dir /home/ttstack/images
sudo tt image create alpine-cloud --image-dir /home/ttstack/images
```

## Image Formats by Engine

### Docker / Podman

Docker images are **container images** pulled from registries or built locally.
They are **not** stored in the TTstack image directory — they live in the
Docker/Podman image store.

**Creating a Docker image:**
```bash
# Pull from a registry
docker pull alpine:latest

# Or build locally
cat > Dockerfile <<'EOF'
FROM scratch
COPY myapp /app
CMD ["/app"]
EOF
docker build -t my-image .
```

**Using with TTstack:**
```bash
tt env create myenv --image alpine --engine docker --port 80
```

**Key points:**
- The `--image` name must match a locally available Docker/Podman image
- Docker images do NOT need to exist in `image_dir`
- Port mappings are handled by Docker's `-p` flag
- Docker manages its own networking (no TAP devices or bridge needed)

### Firecracker

Firecracker images are **directories** containing two files:
- `vmlinux` — uncompressed Linux kernel with virtio_mmio built-in
- `rootfs.ext4` — ext4 filesystem image used as the root device

**Image directory structure:**
```
/home/ttstack/images/
  └── fc-alpine/
      ├── vmlinux        # ~21MB kernel
      └── rootfs.ext4    # Root filesystem
```

**Creating manually:**

1. **Get a Firecracker-compatible kernel:**
   ```bash
   # Option A: Download pre-built (recommended)
   curl -sL https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/kernels/vmlinux.bin \
     -o /home/ttstack/images/fc-alpine/vmlinux

   # Option B: Build from source with virtio_mmio=y
   # (The stock Debian kernel won't work — it has virtio_mmio as a module)
   ```

2. **Create the rootfs:**
   ```bash
   dd if=/dev/zero of=rootfs.ext4 bs=1M count=128
   mkfs.ext4 rootfs.ext4
   mkdir /tmp/mnt && mount -o loop rootfs.ext4 /tmp/mnt

   # Populate with your desired userspace (Alpine, BusyBox, etc.)
   # At minimum: /sbin/init (or /init) must exist
   mkdir -p /tmp/mnt/{bin,sbin,etc,proc,sys,dev}
   # ... install packages or copy a static init binary ...

   umount /tmp/mnt
   ```

**Important notes:**
- The kernel MUST have `virtio_mmio` built-in (not as a module)
- The stock Debian/Ubuntu kernel will NOT work — use the Firecracker pre-built kernel
- Boot args: `console=ttyS0 reboot=k panic=1 pci=off`; the rootfs is set via `is_root_device: true`
- Firecracker VMs use TAP devices on the `tt0` bridge for networking
- Pause/resume is supported via the Firecracker API

### QEMU / KVM

QEMU images are **qcow2 disk image files** containing a bootable filesystem.

**Image location:**
```
/home/ttstack/images/
  └── qemu-test          # Single qcow2 file (for raw storage)
```

When using ZFS or Btrfs storage, the image is a directory containing the qcow2:
```
/home/ttstack/images/
  └── qemu-test/
      └── disk.qcow2     # The actual disk image
```

**Creating manually:**

```bash
# Create qcow2 image
qemu-img create -f qcow2 /home/ttstack/images/qemu-test 256M

# Mount via NBD and populate
modprobe nbd max_part=8
qemu-nbd --connect=/dev/nbd0 /home/ttstack/images/qemu-test
mkfs.ext4 /dev/nbd0
mount /dev/nbd0 /tmp/mnt

# Populate rootfs
mkdir -p /tmp/mnt/{bin,sbin,etc,proc,sys,dev}
# ... install packages or copy a static init binary ...

umount /tmp/mnt
qemu-nbd --disconnect /dev/nbd0
```

**Important notes:**
- QEMU uses KVM acceleration (`-enable-kvm`), so `/dev/kvm` must exist
- The disk is attached as virtio (`if=virtio`), so the guest kernel needs virtio drivers
- QEMU uses TAP devices on the `tt0` bridge for networking
- Stop/start uses the QEMU monitor (`stop`/`cont` commands) — the process stays alive

## Storage Backend Considerations

### Raw (default)

Images are plain files or directories. On Linux, cloning uses
`cp --reflink=auto` (CoW if the filesystem supports it, full copy
otherwise). On FreeBSD, plain `cp -a` is used.

```bash
# No special setup needed — just place images in image_dir
cp my-image.qcow2 /home/ttstack/images/qemu-test
```

### ZFS

Images must be **ZFS datasets** (child datasets of the image pool).

```bash
# Setup
zpool create ttpool /dev/nvmeXnYpZ
zfs create ttpool/images
zfs create ttpool/runtime

# Create image as a dataset
zfs create ttpool/images/qemu-test
cp my-image.qcow2 /ttpool/images/qemu-test/disk.qcow2

# Agent config
tt-agent --image-dir /ttpool/images --runtime-dir /ttpool/runtime --storage zfs
```

Cloning uses `zfs snapshot` + `zfs clone` — instant and space-efficient.

### Btrfs

Images must be **Btrfs subvolumes**.

```bash
# Setup
mkfs.btrfs /dev/nvmeXnYpZ
mount /dev/nvmeXnYpZ /mnt/btrfs-tt
mkdir /mnt/btrfs-tt/{images,runtime}

# Create image as a subvolume
btrfs subvolume create /mnt/btrfs-tt/images/qemu-test
cp my-image.qcow2 /mnt/btrfs-tt/images/qemu-test/disk.qcow2

# Agent config
tt-agent --image-dir /mnt/btrfs-tt/images --runtime-dir /mnt/btrfs-tt/runtime --storage btrfs
```

Cloning uses `btrfs subvolume snapshot` — instant and space-efficient.

## Networking

All VM engines (except Docker) use a shared network topology:

```
Guest VM ←→ TAP device ←→ tt0 bridge (10.10.0.1/16) ←→ NAT (nftables) ←→ Host
```

- Each VM gets a unique IP in the 10.10.0.0/16 range
- Port forwarding: host port → guest port via nftables DNAT (Linux) or PF rdr (FreeBSD)
- The `tt0` bridge and NAT table are created automatically by the agent

Docker containers use Docker's native networking with `-p` port publishing.

FreeBSD uses PF instead of nftables:

```
Guest VM ←→ TAP device ←→ tt0 bridge (10.10.0.1/16) ←→ NAT (PF) ←→ Host
```

## Creating a FreeBSD Test VM on Linux

Running FreeBSD inside QEMU on a Linux host is useful for testing the
FreeBSD agent, controller, and CLI. This section documents the
non-interactive approach, including common pitfalls.

### Recommended: mfsBSD Live ISO

[mfsBSD](https://mfsbsd.vx.sk/) is a FreeBSD live CD that runs entirely
in RAM with SSH enabled out of the box. This is the fastest way to get
a working FreeBSD environment for build/test purposes.

```bash
# Download mfsBSD SE (Special Edition includes base packages)
curl -sL https://mfsbsd.vx.sk/files/iso/14/amd64/mfsbsd-se-14.2-RELEASE-amd64.iso \
  -o /tmp/mfsbsd.iso

# Create a work disk for persistent storage
qemu-img create -f qcow2 /tmp/fbsd-work.qcow2 20G

# Boot the VM (8GB RAM recommended for compiling Rust)
qemu-system-x86_64 -enable-kvm -m 8192 -smp 4 \
  -drive file=/tmp/fbsd-work.qcow2,format=qcow2,if=virtio \
  -cdrom /tmp/mfsbsd.iso -boot d \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -vnc none -daemonize

# SSH in (default root password: mfsroot)
sshpass -p 'mfsroot' ssh -p 2222 root@127.0.0.1
```

### Installing FreeBSD to Disk from mfsBSD

mfsBSD runs in RAM so installed packages are lost on reboot. For
persistent use, install FreeBSD to the work disk:

```bash
# Inside the mfsBSD VM:
gpart create -s gpt vtbd0
gpart add -t freebsd-boot -s 512k vtbd0
gpart add -t freebsd-ufs -l rootfs vtbd0
gpart bootcode -b /boot/pmbr -p /boot/gptboot -i 1 vtbd0
newfs -U /dev/vtbd0p2

# Mount and extract base + kernel
mkdir -p /rw/disk && mount /dev/vtbd0p2 /rw/disk
cd /rw/disk
fetch -o - https://download.freebsd.org/releases/amd64/14.3-RELEASE/base.txz | tar xf -
fetch -o - https://download.freebsd.org/releases/amd64/14.3-RELEASE/kernel.txz | tar xf -

# Configure the installed system
echo '/dev/vtbd0p2  /  ufs  rw  1  1' > etc/fstab
cat > etc/rc.conf <<'RCEOF'
hostname="fbsd-test"
ifconfig_vtnet0="DHCP"
sshd_enable="YES"
sendmail_enable="NONE"
RCEOF
echo 'mfsroot' | chroot /rw/disk pw usermod root -h 0
sed -i '' 's/^#PermitRootLogin .*/PermitRootLogin yes/' etc/ssh/sshd_config
echo 'nameserver 10.0.2.3' > etc/resolv.conf

umount /rw/disk
```

Then reboot the VM without the `-cdrom` and `-boot d` flags to boot
from disk.

### Common Pitfalls

| Problem | Cause | Solution |
|---------|-------|----------|
| FreeBSD cloud images won't accept SSH | Root login disabled, no password auth, cloud-init issues | Use mfsBSD instead |
| `virt-customize` fails on FreeBSD images | libguestfs cannot write to UFS filesystems | Use mfsBSD or manual install |
| mfsBSD `pkg install` fails with "No error" | pkg 2.1.0 on mfsBSD 14.2 cannot handle zstd-packed repos | Install FreeBSD to disk for a newer pkg |
| `cp --reflink=auto` fails on FreeBSD | GNU option not available | Fixed in TTstack — FreeBSD uses `cp -a` |
| `ifconfig bridge create` names bridge `bridge0` | FreeBSD auto-assigns names | Fixed in TTstack — uses `ifconfig bridge create name tt0` |
| `ifconfig <tap> create` fails for custom names | Must specify type first | Fixed in TTstack — uses `ifconfig tap create name <tap>` |
| Bhyve TAP name mismatch | Old code used `tap-{id}` instead of hashed name | Fixed in TTstack — uses `net::tap_name()` |
| OOM during Rust compilation on mfsBSD | mfsBSD runs in RAM; 4GB is insufficient | Use 8GB+ RAM or install to disk |
