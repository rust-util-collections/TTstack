# Kernel

## Configs MUST be Built In

#### file system

- CONFIG_EXT4_FS

#### virtio basic

- CONFIG_VIRTIO_PCI
- CONFIG_VIRTIO_INPUT
- CONFIG_VIRTIO_MMIO
- CONFIG_VIRTIO_MMIO_CMDLINE_DEVICES

#### virtio block device

- CONFIG_VIRTIO_BLK

#### virtio network device

- CONFIG_VSOCKETS
- CONFIG_VIRTIO_VSOCKETS
- CONFIG_VIRTIO_NET

#### make script[s] exectuable

- BINFMT_MISC

## Configs MUST be disabled

#### accept stripped modular[s]

- CONFIG_MODULE_SIG
