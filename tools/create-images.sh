#!/usr/bin/env bash
#
# TTstack guest image creation script
#
# Creates ready-to-use VM images for each supported engine:
#   - Docker:      Pulls or builds a container image
#   - Firecracker: Creates vmlinux + rootfs.ext4 in a directory
#   - QEMU:        Creates a qcow2 disk image
#
# Usage:
#   ./tools/create-images.sh <engine> <image-name> [image-dir]
#
# Examples:
#   ./tools/create-images.sh firecracker fc-alpine /home/ttstack/images
#   ./tools/create-images.sh qemu        qemu-test /home/ttstack/images
#   ./tools/create-images.sh docker      alpine    # Docker images use registry

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

log() { echo "[create-images] $*"; }
err() { echo "[create-images] ERROR: $*" >&2; exit 1; }

# Firecracker kernel URL (pre-built with virtio_mmio built-in)
FC_KERNEL_URL="https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/kernels/vmlinux.bin"

# ── Firecracker Image ────────────────────────────────────────────────

create_firecracker_image() {
    local name="$1"
    local image_dir="$2"
    local target="$image_dir/$name"

    log "Creating Firecracker image: $name"
    mkdir -p "$target"

    # Download Firecracker-compatible kernel if not present
    if [ ! -f "$target/vmlinux" ]; then
        log "Downloading Firecracker kernel..."
        curl -sL "$FC_KERNEL_URL" -o "$target/vmlinux"
        log "Kernel downloaded ($(du -h "$target/vmlinux" | cut -f1))"
    else
        log "Kernel already exists"
    fi

    # Create rootfs
    local rootfs_size="${ROOTFS_SIZE_MB:-128}"
    log "Creating rootfs (${rootfs_size}MB)..."
    dd if=/dev/zero of="$target/rootfs.ext4" bs=1M count="$rootfs_size" 2>/dev/null
    mkfs.ext4 -q "$target/rootfs.ext4"

    # Mount and populate
    local mnt
    mnt=$(mktemp -d)
    mount -o loop "$target/rootfs.ext4" "$mnt"

    mkdir -p "$mnt"/{bin,sbin,etc,proc,sys,dev,tmp,run,var/log,root}

    # Create minimal init that mounts essential filesystems
    cat > "$mnt/init_src.c" <<'INITEOF'
#include <unistd.h>
#include <sys/mount.h>
#include <stdio.h>
#include <sys/stat.h>
int main() {
    mount("proc", "/proc", "proc", 0, NULL);
    mount("sysfs", "/sys", "sysfs", 0, NULL);
    mount("devtmpfs", "/dev", "devtmpfs", 0, NULL);
    mkdir("/dev/pts", 0755);
    mount("devpts", "/dev/pts", "devpts", 0, NULL);
    sethostname("ttstack", 7);
    printf("TTstack Firecracker guest [%s] booted OK\n", "$name");
    fflush(stdout);
    while(1) sleep(3600);
    return 0;
}
INITEOF
    gcc -static -o "$mnt/sbin/init" "$mnt/init_src.c"
    rm "$mnt/init_src.c"
    ln -sf /sbin/init "$mnt/init"

    # Set up basic /etc
    echo "root:x:0:0:root:/root:/bin/sh" > "$mnt/etc/passwd"
    echo "root:x:0:" > "$mnt/etc/group"
    echo "$name" > "$mnt/etc/hostname"

    umount "$mnt"
    rmdir "$mnt"

    log "Firecracker image ready: $target/"
    log "  vmlinux:     $(du -h "$target/vmlinux" | cut -f1)"
    log "  rootfs.ext4: $(du -h "$target/rootfs.ext4" | cut -f1)"
}

# ── QEMU Image ───────────────────────────────────────────────────────

create_qemu_image() {
    local name="$1"
    local image_dir="$2"
    local target="$image_dir/$name"
    local disk_size="${DISK_SIZE_MB:-256}"

    log "Creating QEMU image: $name (${disk_size}MB)"

    # Check for qemu-img and qemu-nbd
    command -v qemu-img >/dev/null || err "qemu-img not found (install qemu-utils)"
    command -v qemu-nbd >/dev/null || err "qemu-nbd not found (install qemu-utils)"

    # Create qcow2 image
    qemu-img create -f qcow2 "$target" "${disk_size}M"

    # Load nbd module
    modprobe nbd max_part=8 2>/dev/null || true

    # Find available nbd device (one with size 0 = not connected)
    local nbd=""
    for dev in /dev/nbd{0..15}; do
        if [ -b "$dev" ]; then
            local size
            size=$(cat "/sys/class/block/$(basename "$dev")/size" 2>/dev/null || echo "0")
            if [ "$size" = "0" ]; then
                nbd="$dev"
                break
            fi
        fi
    done
    [ -n "$nbd" ] || err "no available nbd device"

    # Connect and format
    qemu-nbd --connect="$nbd" "$target"
    sleep 1
    mkfs.ext4 -q "$nbd"

    # Mount and populate
    local mnt
    mnt=$(mktemp -d)
    mount "$nbd" "$mnt"

    mkdir -p "$mnt"/{bin,sbin,etc,proc,sys,dev,tmp,run,var/log,root}

    # Create minimal init
    cat > "$mnt/init_src.c" <<'INITEOF'
#include <unistd.h>
#include <sys/mount.h>
#include <stdio.h>
#include <sys/stat.h>
int main() {
    mount("proc", "/proc", "proc", 0, NULL);
    mount("sysfs", "/sys", "sysfs", 0, NULL);
    mount("devtmpfs", "/dev", "devtmpfs", 0, NULL);
    sethostname("ttstack", 7);
    printf("TTstack QEMU guest booted OK\n");
    fflush(stdout);
    while(1) sleep(3600);
    return 0;
}
INITEOF
    gcc -static -o "$mnt/sbin/init" "$mnt/init_src.c"
    rm "$mnt/init_src.c"
    ln -sf /sbin/init "$mnt/init"

    echo "root:x:0:0:root:/root:/bin/sh" > "$mnt/etc/passwd"
    echo "root:x:0:" > "$mnt/etc/group"

    umount "$mnt"
    rmdir "$mnt"
    qemu-nbd --disconnect "$nbd"

    log "QEMU image ready: $target ($(du -h "$target" | cut -f1))"
}

# ── Docker Image ─────────────────────────────────────────────────────

create_docker_image() {
    local name="$1"

    log "Creating Docker image: $name"

    command -v docker >/dev/null || command -v podman >/dev/null \
        || err "neither docker nor podman found"

    local rt="docker"
    command -v docker >/dev/null || rt="podman"

    local tmpdir
    tmpdir=$(mktemp -d)

    # Create minimal image with a sleep process
    cat > "$tmpdir/init.c" <<'INITEOF'
#include <unistd.h>
int main() { while(1) sleep(3600); return 0; }
INITEOF
    gcc -static -o "$tmpdir/init" "$tmpdir/init.c"
    rm "$tmpdir/init.c"

    cat > "$tmpdir/Dockerfile" <<'DEOF'
FROM scratch
COPY init /init
CMD ["/init"]
DEOF

    $rt build -t "$name" "$tmpdir"
    rm -rf "$tmpdir"

    log "Docker image ready: $name"
    log "  Use with: tt env create <env-name> --image $name --engine docker"
}

# ── ZFS Image ────────────────────────────────────────────────────────
# For ZFS storage backend, images must be ZFS datasets.
# This creates a dataset and populates it with engine-specific files.

create_zfs_image() {
    local engine="$1"
    local name="$2"
    local image_dir="$3"

    # image_dir should be a ZFS dataset path (e.g. ttpool/images)
    local ds_base
    ds_base=$(echo "$image_dir" | sed 's|^/||')

    log "Creating ZFS dataset: $ds_base/$name"
    zfs create "$ds_base/$name" 2>/dev/null || log "Dataset already exists"

    local mountpoint
    mountpoint=$(zfs get -H -o value mountpoint "$ds_base/$name")

    case "$engine" in
        firecracker)
            create_firecracker_image "$name" "$(dirname "$mountpoint")"
            ;;
        qemu)
            create_qemu_image "disk.qcow2" "$mountpoint"
            mv "$mountpoint/disk.qcow2" "$mountpoint/disk.qcow2" 2>/dev/null || true
            ;;
    esac

    log "ZFS image ready: $ds_base/$name (mountpoint: $mountpoint)"
}

# ── Main ─────────────────────────────────────────────────────────────

usage() {
    cat <<EOF
TTstack Guest Image Creator

Usage:
  $0 <engine> <image-name> [image-dir]

Engines:
  firecracker   Create vmlinux + rootfs.ext4 in a directory
  qemu          Create a qcow2 disk image
  docker        Build a minimal container image

Options (via environment):
  ROOTFS_SIZE_MB   Firecracker rootfs size (default: 128)
  DISK_SIZE_MB     QEMU disk size (default: 256)

Examples:
  $0 firecracker fc-alpine /home/ttstack/images
  $0 qemu qemu-test /home/ttstack/images
  $0 docker tt-test-alpine
EOF
    exit 1
}

main() {
    local engine="${1:-}"
    local name="${2:-}"
    local image_dir="${3:-/home/ttstack/images}"

    [ -n "$engine" ] && [ -n "$name" ] || usage

    case "$engine" in
        firecracker)
            [ "$(id -u)" -eq 0 ] || err "must be root (mount/losetup needed)"
            create_firecracker_image "$name" "$image_dir"
            ;;
        qemu)
            [ "$(id -u)" -eq 0 ] || err "must be root (nbd/mount needed)"
            create_qemu_image "$name" "$image_dir"
            ;;
        docker)
            create_docker_image "$name"
            ;;
        *)
            err "unknown engine: $engine (use: firecracker, qemu, docker)"
            ;;
    esac
}

main "$@"
