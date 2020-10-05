#!/bin/sh

qemu-system-x86_64 \
    -enable-kvm \
    -cpu host -smp 4,sockets=4,cores=1,threads=1 \
    -m 8192 \
    -net nic -net user,hostfwd=tcp::2222-:22 \
    -drive file=$1,if=none,format=raw,cache=none,id=DISK_111 \
    -device virtio-blk-pci,drive=DISK_111 \
    -vnc :9 \
    -daemonize \
    -boot order=cd

# -drive file=$2,readonly=on,media=cdrom \
