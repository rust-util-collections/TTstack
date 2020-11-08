//!
//! # Qemu Virtual Machine
//!

#[cfg(any(feature = "zfs", feature = "nft"))]
use crate::{asleep, POOL};
use crate::{
    linux::vm::{cmd_exec, util::wait_pid},
    vmimg_path, Vm,
};
#[cfg(feature = "zfs")]
use crate::{CLONE_MARK, ZFS_ROOT};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use std::fs;

#[cfg(feature = "nft")]
pub(super) const BRIDGE: &str = "ttcore-bridge";

lazy_static! {
    static ref IOMMU: &'static str = {
        pnk!(
            fs::read("/proc/cpuinfo")
                .c(d!())
                .and_then(|c| String::from_utf8(c).c(d!()))
                .and_then(|cpuinfo| {
                    if cpuinfo.contains(" svm ") {
                        Ok("amd-iommu")
                    } else if cpuinfo.contains(" vmx ") {
                        Ok("intel-iommu")
                    } else {
                        Err(eg!("Unsupported platform!"))
                    }
                })
        )
    };
}

/// 设置基本运行环境
/// firecracker also need this!
#[cfg(feature = "nft")]
pub(super) fn init() -> Result<()> {
    fs::write("/proc/sys/net/ipv4/ip_forward", "1")
        .c(d!())
        .and_then(|_| cmd_exec("modprobe", &["tun"]).c(d!()))
        .and_then(|_| {
            cmd_exec("ip", &["addr", "flush", "dev", BRIDGE])
                .c(d!())
                .or_else(|e| {
                    cmd_exec("ip", &["link", "add", BRIDGE, "type", "bridge"])
                        .c(d!(e))
                })
        })
        .and_then(|_| {
            cmd_exec("ip", &["addr", "add", "10.0.0.1/8", "dev", BRIDGE])
                .c(d!())
        })
        .and_then(|_| cmd_exec("ip", &["link", "set", BRIDGE, "up"]).c(d!()))
        .map(|_| {
            (0..1000).for_each(|n| {
                omit!(cmd_exec("ip", &["link", "del", &format!("TAP-{}", n)]));
            });
        })
}

#[inline(always)]
#[cfg(not(feature = "nft"))]
pub(super) fn init() -> Result<()> {
    Ok(())
}

#[cfg(feature = "nft")]
pub(super) fn start(vm: &Vm) -> Result<()> {
    let cpu = vm.cpu_num.to_string();
    let mem = vm.mem_size.to_string();

    let netdev = format!(
        "tap,ifname=TAP-{0},script=no,downscript=no,id=NET_{0}",
        vm.id
    );

    const WIDTH: usize = 2;
    let netdev_device = format!(
        "virtio-net-pci,mac=52:54:00:11:{:>0width$x}:{:>0width$x},netdev=NET_{}",
        vm.id / 256,
        vm.id % 256,
        vm.id,
        width = WIDTH,
    );

    let (disk, disk_device) = gen_disk_info(vm);
    let uuid = if vm.rand_uuid {
        gen_vm_uuid().c(d!())?
    } else {
        "5ce41b72-0e2e-48f9-8422-7647b557aba8".to_owned()
    };

    let args = &[
        "-enable-kvm",
        "-machine",
        "q35,accel=kvm",
        "-device",
        &IOMMU,
        "-cpu",
        "host",
        "-smp",
        cpu.as_str(),
        "-m",
        mem.as_str(),
        "-netdev",
        netdev.as_str(),
        "-device",
        netdev_device.as_str(),
        "-drive",
        disk.as_str(),
        "-device",
        disk_device.as_str(),
        "-boot",
        "order=cd",
        "-vnc",
        &format!(":{}", vm.id),
        "-uuid",
        &uuid,
        "-daemonize",
    ];

    cmd_exec("qemu-system-x86_64", dbg!(args))
        .map(|_| {
            // Qemu daemonize 模式
            // 会产生一个需要接管的父进程
            wait_pid();
        })
        .c(d!())
        .and_then(|_| set_tap(vm).c(d!()))
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn set_tap(vm: &Vm) -> Result<()> {
    let tap = format!("TAP-{}", vm.id);
    cmd_exec("ip", &["link", "set", &tap, "master", &BRIDGE])
        .c(d!())
        .and_then(|_| cmd_exec("ip", &["link", "set", &tap, "up"]).c(d!()))
}

#[cfg(not(feature = "nft"))]
pub(super) fn start(vm: &Vm) -> Result<()> {
    let cpu = vm.cpu_num.to_string();
    let mem = vm.mem_size.to_string();

    let netdevice = format!("virtio-net,netdev=NET_{}", vm.id);
    let netdev = {
        let mut base = vct![format!("user,id=NET_{}", vm.id)];
        vm.port_map.iter().for_each(|(vmport, pubport)| {
            base.push(format!(",hostfwd=tcp::{}-:{}", pubport, vmport));
            base.push(format!(",hostfwd=udp::{}-:{}", pubport, vmport));
        });
        base.join("")
    };

    let (disk, disk_device) = gen_disk_info(vm);
    let uuid = gen_vm_uuid().c(d!())?;

    let args = &[
        "-enable-kvm",
        "-machine",
        "q35,accel=kvm",
        "-device",
        &IOMMU,
        "-cpu",
        "host",
        "-smp",
        cpu.as_str(),
        "-m",
        mem.as_str(),
        "-device",
        &netdevice,
        "-netdev",
        &netdev,
        "-drive",
        disk.as_str(),
        "-device",
        disk_device.as_str(),
        "-boot",
        "order=cd",
        "-vnc",
        &format!(":{}", vm.id),
        "-uuid",
        &uuid,
        "-daemonize",
    ];

    cmd_exec("qemu-system-x86_64", dbg!(args))
        .map(|_| {
            // Qemu daemonize 模式
            // 会产生一个需要接管的父进程
            wait_pid();
        })
        .c(d!())
}

// for upper caller
#[inline(always)]
pub(crate) fn pre_starter(vm: &Vm) -> Result<()> {
    if !vm.image_cached {
        create_img(vm).c(d!())?;
    }

    #[cfg(feature = "nft")]
    create_tap(&format!("TAP-{}", vm.id)).c(d!())?;

    Ok(())
}

#[cfg(feature = "nft")]
#[inline(always)]
fn create_tap(tap: &str) -> Result<()> {
    cmd_exec("ip", &["tuntap", "add", &tap, "mode", "tap"]).c(d!())
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn remove_tap(vm: &Vm) -> Result<()> {
    let tap = format!("TAP-{}", vm.id);

    // 立即执行会失败
    POOL.spawn_ok(async move {
        asleep(5).await;
        info_omit!(
            cmd_exec("ip", &["tuntap", "del", &tap, "mode", "tap"]).c(d!())
        );
    });

    Ok(())
}

#[cfg(feature = "zfs")]
pub(crate) fn create_img(vm: &Vm) -> Result<()> {
    let arg = format!(
        "zfs clone -o volmode=dev {root}/{os}@base {root}/{clone_mark}{id}",
        root = *ZFS_ROOT,
        os = vm
            .image_path
            .file_name()
            .ok_or(eg!())?
            .to_str()
            .ok_or(eg!())?,
        clone_mark = CLONE_MARK,
        id = vm.id,
    );

    cmd_exec("sh", &["-c", &arg]).c(d!())
}

#[cfg(feature = "zfs")]
fn gen_disk_info(vm: &Vm) -> (String, String) {
    let disk = format!(
        "file={img},if=none,format=raw,cache=none,id=DISK_{id}",
        img = vmimg_path(vm).to_string_lossy(),
        id = vm.id,
    );
    let disk_device = format!("virtio-blk-pci,drive=DISK_{}", vm.id);
    (disk, disk_device)
}

// 命名格式为: ${CLONE_MARK}_VmId
#[cfg(feature = "zfs")]
#[inline(always)]
pub(super) fn remove_image(vm: &Vm) -> Result<()> {
    let arg = format!(
        "zfs destroy {root}/{clone_mark}{id}",
        root = *ZFS_ROOT,
        clone_mark = CLONE_MARK,
        id = vm.id
    );

    // zfs destroy 立即执行会失败
    POOL.spawn_ok(async move {
        asleep(5).await;
        info_omit!(cmd_exec("sh", &["-c", &arg]));
    });

    Ok(())
}

// 基于基础镜像, 创建临时运行镜像,
// 命名格式为: ${CLONE_MARK}_VmId
#[cfg(all(feature = "cow", not(feature = "zfs")))]
pub(crate) fn create_img(vm: &Vm) -> Result<()> {
    let vmimg_path = vmimg_path(vm).to_string_lossy().into_owned();
    let args = &[
        "--reflink=always",
        &vm.image_path.to_string_lossy(),
        &vmimg_path,
    ];

    omit!(fs::remove_file(&vmimg_path));
    cmd_exec("cp", args).c(d!())
}

#[cfg(all(feature = "cow", not(feature = "zfs")))]
fn gen_disk_info(vm: &Vm) -> (String, String) {
    let disk = format!(
        "file={},if=none,media=disk,id=DISK_{}",
        vmimg_path(vm).to_string_lossy(),
        vm.id,
    );
    let disk_device = format!("virtio-blk-pci,drive=DISK_{}", vm.id);

    (disk, disk_device)
}

// 基于基础镜像, 创建临时运行镜像,
// 命名格式为: ${CLONE_MARK}_VmId
#[cfg(not(any(feature = "cow", feature = "zfs")))]
pub(crate) fn create_img(vm: &Vm) -> Result<()> {
    let vmimg_path = vmimg_path(vm).to_string_lossy().into_owned();

    // **注意**:
    // 若指定了 size 选项, 则必须 >= 原始基础镜像,
    // 否则将启动失败, 此处直接不指定递增镜像的大小.
    let option = format!("backing_file={}", vm.image_path.to_string_lossy());

    let args = &["create", "-f", "qcow2", "-o", option.as_str(), &vmimg_path];

    omit!(fs::remove_file(&vmimg_path));
    cmd_exec("qemu-img", args).c(d!())
}

#[cfg(not(any(feature = "cow", feature = "zfs")))]
fn gen_disk_info(vm: &Vm) -> (String, String) {
    let disk = format!(
        "file={},if=none,media=disk,id=DISK_{}",
        vmimg_path(vm).to_string_lossy(),
        vm.id
    );
    let disk_device = format!("virtio-blk-pci,drive=DISK_{}", vm.id);

    (disk, disk_device)
}

// 命名格式为: ${CLONE_MARK}_VmId
#[cfg(not(feature = "zfs"))]
#[inline(always)]
pub(super) fn remove_image(vm: &Vm) -> Result<()> {
    fs::remove_file(vmimg_path(vm)).c(d!())
}

fn gen_vm_uuid() -> Result<String> {
    fs::read("/proc/sys/kernel/random/uuid")
        .c(d!())
        .and_then(|mut uuid| {
            // drop the '\n' at the end
            uuid.pop();
            String::from_utf8(uuid).c(d!())
        })
}
