//!
//! # How it works
//!
//! ## rootfs
//!
//! - use the minimal apline-3.12 docker-image as base image
//!     0. [A] `dd if=/dev/zero of=firecracker.ext4 bs=1M count=$[40 * 1024]`
//!     0. [B] `zfs create -V 40g zroot/tt/firecracker`
//!     1. [A] `mkfs.ext4 firecracker.ext4`
//!     1. [B] `mkfs.ext4 /dev/zvol/zroot/tt/firecracker`
//!     2. `mkdir /tmp/rootfs`
//!     3. [A] `mount rootfs.ext4 /tmp/rootfs`
//!     3. [B] `mount /dev/zvol/zroot/tt/firecracker /tmp/rootfs`
//!     4. [A] `docker run -it --rm -v /tmp/rootfs:/rootfs --privileged alpine`
//!     4. [B] `podman run -it --rm -v /tmp/rootfs:/rootfs --privileged alpine`
//!     5. `sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories`
//!     6. `apk add openrc util-linux openssh bash`
//!     7. `ln -s agetty /etc/init.d/agetty.ttyS0`
//!     8. `echo ttyS0 > /etc/securetty`
//!     9. `rc-update add agetty.ttyS0 default`
//!     10. `rc-update add local default`
//!     11. `rc-update add sshd default`
//!     12. `rc-update add devfs boot`
//!     13. `rc-update add procfs boot`
//!     14. `rc-update add sysfs boot`
//!     15. `for d in bin etc lib root sbin usr; do tar -c "/${d}" | tar -xC /rootfs; done`
//!     16. `for d in dev proc run sys var tmp; do mkdir /rootfs/${d}; done`
//!     17. `mkdir -p /rootfs/var/cache/apk`
//!     18. `exit`
//!     19. `umount /tmp/rootfs`
//! - create image for VM: [A] use zfs' snapshot/clone feature
//!     - `zfs snapshot zroot/tt/firecracker zroot/tt/firecracker@base`    # BaseImage
//!     - `zfs clone zroot/tt/firecracker@base zroot/tt/${OS}`
//!     - `zfs snapshot zroot/tt/${OS} zroot/tt/${OS}@base`                # SnapshotImage
//!     - `zfs clone zroot/tt/${OS}@base zroot/tt/clone_${VmId}`           # RuntimeImage
//! - create image for VM: [B] use btrfs' COW feature
//!     - **NOTE**: do NOT use this mode in 100+ VMs scene, IO performance will be very poor
//!     - `cp --reflink=always $BaseImage $SnapshotImage`       # SnapshotImage
//!     - `cp --reflink=always $SnapshotImage $RuntimeImage`    # RuntimeImage
//! - pack every version of kernel-modules in '/lib/modules/' dir
//! - pack all 'os-release/redhat-release/centos-release' in '/etc/all_os_info/' dir
//!     - add a OS-mark prefix to every origin file
//!         - eg: `/etc/os_release_info/centos-7.x:os-release`
//!         - eg: `/etc/os_release_info/centos-7.8:centos-release`
//!     - User can create symlink to standard-path at runtime
//!         - eg: 'ln -sv /etc/os_release_info/centos-7.x:os-release /etc/os-release'
//!
//! ## kernel
//!
//! - compile these options into kernel(on standard `CentOS` platform, they are modulars)
//!     - BINFMT_MISC
//!     - CONFIG_EXT4_FS
//!     - CONFIG_VIRTIO_PCI
//!     - CONFIG_VIRTIO_INPUT
//!     - CONFIG_VIRTIO_MMIO
//!     - CONFIG_VIRTIO_MMIO_CMDLINE_DEVICES
//!     - CONFIG_VIRTIO_BLK
//!     - CONFIG_VSOCKETS
//!     - CONFIG_VIRTIO_VSOCKETS
//!     - CONFIG_VIRTIO_NET
//! - disable these options
//!     - CONFIG_MODULE_SIG # allow stripped modular
//! - $KernelPath = /tt/kernel/$SnapshotImagePath
//!     - eg: '/tt/kernel/fire:centos-7.8:3.10.el7.x86_64'
//! - pack every version of kernel-modules in '/lib/modules/' dir
//!
//! ## network
//!
//! **depend on `qemu::init()` !**
//!

use crate::linux::vm::{cmd_exec_daemonize, engine::qemu, Vm};
use myutil::{err::*, *};
use std::fs;

pub(crate) const LOG_DIR: &str = "/home/firecracker_vm_log";

// - generate $CFG.json, and write it to '/tmp/$VM_ID.json'
//     - $KernelPath = /tt/kernel/$SnapshotImagePath
//     - tt is running in a private 'MNT' namespace
//     - '/tmp' is mounted as private in the namespace
//     - so there is no-risk to conflict with other processes
// - `firecracker --no-api --seccomp-level 0 --config-file $CFG.json`
pub(crate) fn start(vm: &Vm) -> Result<()> {
    let cfg = vmcfg::gen(vm).c(d!())?;
    let cfg_file = format!("/tmp/{}.json", vm.id);
    let log_file = format!("{}/vm_{}", LOG_DIR, vm.id);

    let arg = &[
        "--no-api",
        "--seccomp-level",
        "0",
        "--config-file",
        &cfg_file,
        "--log-path",
        &log_file,
    ];

    fs::File::create(&log_file)
        .c(d!())
        .and_then(|_| fs::write(&cfg_file, &cfg).c(d!()))
        .and_then(|_| {
            println!("{}", &cfg);
            cmd_exec_daemonize("/usr/sbin/firecracker", dbg!(arg)).c(d!())
        })
        .and_then(|_| set_tap(vm).c(d!()))
}

// for upper caller
#[inline(always)]
pub(crate) fn pre_starter(vm: &Vm) -> Result<()> {
    qemu::pre_starter(vm).c(d!())
}

// 运行时镜像创建/清理的方式,
// 与 Qemu 相同, 命名格式为: ${CLONE_MARK}_VmId
#[inline(always)]
pub(crate) fn remove_image(vm: &Vm) -> Result<()> {
    qemu::remove_image(vm).c(d!())
}

#[inline(always)]
pub(crate) fn remove_tap(vm: &Vm) -> Result<()> {
    qemu::remove_tap(vm).c(d!())
}

#[inline(always)]
pub(super) fn set_tap(vm: &Vm) -> Result<()> {
    qemu::set_tap(vm).c(d!())
}

mod vmcfg {
    use crate::{linux::vm::Vm, vmimg_path};
    use myutil::{err::*, *};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Cfg {
        #[serde(rename = "boot-source")]
        boot_source: BootSource,
        #[serde(rename = "drives")]
        disks: Vec<Disk>,
        #[serde(rename = "network-interfaces")]
        network_interfaces: Vec<NetworkInterface>,
        #[serde(rename = "machine-config")]
        machine_config: MachineConfig,
    }

    #[derive(Serialize)]
    struct MachineConfig {
        vcpu_count: i32,
        mem_size_mib: i32,
        ht_enabled: bool,
    }

    impl MachineConfig {
        fn new(vcpu_count: i32, mem_size_mib: i32) -> Self {
            MachineConfig {
                vcpu_count,
                mem_size_mib,
                ht_enabled: false,
            }
        }
    }

    #[derive(Serialize)]
    struct BootSource {
        kernel_image_path: String,
        boot_args: &'static str,
    }

    impl BootSource {
        fn new(vm: &Vm) -> Result<Self> {
            let kernel_image_path =
                format!("/tt/kernel/{}", vm.image_path.to_string_lossy());

            Ok(BootSource {
                kernel_image_path,
                // boot_args: "console=ttyS0 reboot=k panic=1 pci=off",
                boot_args: "reboot=k panic=1 pci=off",
            })
        }
    }

    #[derive(Serialize)]
    struct Disk {
        drive_id: String,
        path_on_host: String,
        is_root_device: bool,
        is_read_only: bool,
    }

    impl Disk {
        fn new(vm: &Vm) -> Self {
            Disk {
                drive_id: format!("rootfs.{}", vm.id),
                path_on_host: vmimg_path(vm).to_string_lossy().into_owned(),
                is_root_device: true,
                is_read_only: false,
            }
        }
    }

    #[derive(Serialize)]
    struct NetworkInterface {
        iface_id: &'static str,
        guest_mac: String,
        host_dev_name: String,
    }

    impl NetworkInterface {
        fn new(vm: &Vm) -> Self {
            const WIDTH: usize = 2;

            let guest_mac = format!(
                "52:54:00:11:{:>0width$x}:{:>0width$x}",
                vm.id() / 256,
                vm.id() % 256,
                width = WIDTH
            );
            let host_dev_name = format!("TAP-{}", vm.id);

            NetworkInterface {
                iface_id: "eth0",
                guest_mac,
                host_dev_name,
            }
        }
    }

    pub(super) fn gen(vm: &Vm) -> Result<String> {
        let machine_config = MachineConfig::new(vm.cpu_num, vm.mem_size);
        let boot_source = BootSource::new(vm).c(d!())?;
        let disks = vct![Disk::new(vm)];
        let network_interfaces = vct![NetworkInterface::new(vm)];

        serde_json::to_string(&Cfg {
            machine_config,
            boot_source,
            disks,
            network_interfaces,
        })
        .c(d!())
    }
}
