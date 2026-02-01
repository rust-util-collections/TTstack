# Guest Image Guide

TTstack supports multiple VM engines, each requiring its own image format.
This guide covers how to create, manage, and deploy guest images for each engine.

## Quick Start

Use the automated script:

```bash
# Firecracker microVM image (vmlinux + rootfs)
sudo ./tools/create-images.sh firecracker fc-alpine /home/ttstack/images

# QEMU/KVM disk image (qcow2)
sudo ./tools/create-images.sh qemu qemu-test /home/ttstack/images

# Docker container image (built locally)
./tools/create-images.sh docker tt-test-alpine
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
- Boot args include `root=/dev/vda` — the rootfs appears as `/dev/vda`
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

Images are plain files or directories. Cloning uses `cp --reflink=auto`
(CoW if the filesystem supports it, full copy otherwise).

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
- Port forwarding: host port → guest port via nftables DNAT rules
- The `tt0` bridge and NAT table are created automatically by the agent

Docker containers use Docker's native networking with `-p` port publishing.
