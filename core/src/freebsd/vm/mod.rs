//!
//! # Virtual Machine Mgmt
//!
//! - Bhyve
//! - Jail
//! - ...
//!

use crate::{
    asleep,
    freebsd::{CLONE_MARK, ZFS_ROOT},
    Vm, VmId, POOL,
};
use myutil::err::*;
use myutil::*;
use nix::unistd::{daemon, execv, fork, ForkResult};
use std::{ffi::CString, process};

// 清理旧数据,
// 是否应由启动脚本去做?
pub(super) fn init() -> Result<()> {
    let arg = r"kldload vmm ipfw ipfw_nat if_bridge if_tap 2>/dev/null; sysctl net.link.tap.up_on_open=1 || exit 1; ifconfig bridge0 destroy 2>/dev/null; ifconfig bridge0 create up || exit 1; ifconfig bridge0 inet 10.0.0.1/8 -alias 2>/dev/null; ifconfig bridge0 inet 10.0.0.1/8 alias || exit 1;";

    cmd_exec("sh", &["-c", arg])
        .c(d!())
        .and_then(|_| env_clean().c(d!()))
}

// 清理 VM 环境
pub(super) fn env_clean() -> Result<()> {
    let arg = format!(
        r"for i in `ls /dev/vmm | grep -o '^[0-9]\+'`; do bhyvectl --destroy --vm=$i || exit 1; done; for i in `zfs list -t all | grep -o '{}/{}[0-9]\+'`; do zfs destroy $i || exit 1; done;",
        *ZFS_ROOT, CLONE_MARK
    );

    cmd_exec("sh", &["-c", &arg]).c(d!())
}

#[inline(always)]
pub(crate) fn pre_start(vm: &Vm) -> Result<()> {
    // zfs destroy 动作有延迟,
    // 在 init 中统一清理, 此处不再处理
    let pre_arg = format!(
        r"ifconfig tap{id} destroy 2>/dev/null; ifconfig tap{id} create || exit 1; ifconfig bridge0 addm tap{id} up || exit 1; bhyvectl --destroy --vm={id} 2>/dev/null; zfs clone -o volmode=dev {root}/{os}@base {root}/{clone_mark}{id}",
        id = vm.id(),
        root = *ZFS_ROOT,
        os = vm
            .image_path
            .file_name()
            .ok_or(eg!())?
            .to_str()
            .ok_or(eg!())?,
        clone_mark = CLONE_MARK
    );

    cmd_exec("sh", &["-c", &pre_arg]).c(d!())
}

#[inline(always)]
pub(crate) fn start(vm: &Vm) -> Result<()> {
    bhyve_exec(vm).c(d!())
}

pub(crate) fn bhyve_exec(vm: &Vm) -> Result<()> {
    let id = vm.id().to_string();
    let cpu = vm.cpu_num.to_string();
    let mem = format!("{}M", vm.mem_size);
    let disk =
        format!("2,virtio-blk,/dev/zvol/{}/{}{}", *ZFS_ROOT, CLONE_MARK, &id);

    const WIDTH: usize = 2;
    let nic = format!(
        "3,virtio-net,tap{id},mac=00:be:fa:76:{aa:>0width$x}:{bb:>0width$x}",
        id = &id,
        aa = vm.id() / 256,
        bb = vm.id() % 256,
        width = WIDTH,
    );

    let args = &[
        "-A",
        "-H",
        "-P",
        "-c",
        &cpu,
        "-m",
        &mem,
        "-s",
        "0,hostbridge",
        "-s",
        "1,lpc",
        "-s",
        &disk,
        "-s",
        &nic,
        "-l",
        "bootrom,/usr/local/share/uefi-firmware/BHYVE_UEFI.fd",
        &id,
    ];

    start_vm("/usr/sbin/bhyve", dbg!(args)).c(d!())
}

// 必须后台执行
#[inline(always)]
fn start_vm(cmd: &str, args: &[&str]) -> Result<()> {
    let cmd = gen_cstring(cmd);
    let args = args.iter().map(|arg| gen_cstring(arg)).collect::<Vec<_>>();

    match fork() {
        Ok(ForkResult::Child) => daemon(false, false)
            .c(d!())
            .and_then(|_| {
                execv(
                    &cmd,
                    &args
                        .as_slice()
                        .iter()
                        .map(|arg| arg.as_ref())
                        .collect::<Vec<_>>(),
                )
                .c(d!())
            })
            .map(|_| ()),
        Ok(_) => Ok(()),
        Err(e) => Err(e).c(d!()),
    }
}

#[inline(always)]
fn gen_cstring(s: &str) -> CString {
    unsafe { CString::from_vec_unchecked(s.as_bytes().to_vec()) }
}

// Do nothing on freebsd.
pub(crate) fn zobmie_clean() {}

// 清理过程中出错继续执行
//     - 请理 `/dev/vmm` 下的 VM 名称占用
//     - 清理 VM 的临时 `clone` 镜像
//         - 路径格式为: ${ZFS_ROOT}/${VM_ID}
//     - 清理 tap${VM_ID} 网络设备
#[inline(always)]
pub(crate) fn post_clean(vm: &Vm) {
    let arg = format!(
        r"bhyvectl --destroy --vm={id}; (sleep 2; zfs destroy {root}/{clone_mark}{id}) & ifconfig bridge0 deletem tap{id}; ifconfig tap{id} destroy &",
        root = *ZFS_ROOT,
        id = vm.id(),
        clone_mark = CLONE_MARK
    );

    // zfs destroy 立即执行会失败
    POOL.spawn_ok(async move {
        asleep(5).await;
        info_omit!(cmd_exec("sh", &["-c", dbg!(&arg)]));
    })
}

pub(crate) fn bhyve_stop(vm: VmId) -> Result<()> {
    let arg = format!("bhyvectl --destroy --vm={}", vm);
    cmd_exec("sh", &["-c", &arg]).c(d!())
}

// 执行命令
#[inline(always)]
pub(super) fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;

    if res.status.success() {
        Ok(())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}

#[inline(always)]
pub(crate) fn get_pre_starter(_vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    Ok(pre_start)
}
