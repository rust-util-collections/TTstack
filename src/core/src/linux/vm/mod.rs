//!
//! # Virtual Machine Mgmt
//!
//! - Qemu
//! - FireCracker
//! - ...
//!

pub(crate) mod cgroup;
pub(crate) mod engine;
pub(crate) mod util;

use crate::Vm;
use myutil::err::*;
use myutil::*;
#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
use nix::unistd::{fork, ForkResult};
#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
use std::os::unix::process::CommandExt;
use std::process;
#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
use std::process::Stdio;

#[inline(always)]
pub(crate) fn start(vm: &Vm) -> Result<()> {
    // 1. 首先, 分配 Cgroup 挂载点
    // 2. 之后, 控制进制先进入 Vm 的 Cgroup
    //   - 其创建的 Vm 进程会自动归属于相同的 Cgroup
    //   - 在清理 Vm 进程时要跳过可能存在的控制进程 PID
    cgroup::alloc_mnt_point(vm.id)
        .c(d!())
        .and_then(|_| cgroup::add_vm(vm.id, process::id()).c(d!()))
        .and_then(|_| engine::start(vm).c(d!()))
}

// Avoid this by using "sh -c ..." to start Qemu?
#[inline(always)]
pub(crate) fn zobmie_clean() {
    util::wait_pid()
}

#[inline(always)]
pub(crate) fn post_clean(vm: &Vm) {
    // 停止 Vm 进程及关联的 Cgroup
    info_omit!(cgroup::kill_vm(vm.id));

    // 清理为 Vm 创建的临时 image
    if !vm.image_cached {
        info_omit!(engine::remove_image(vm));
    }

    // 清理为 Vm 创建的 TAP 设备
    #[cfg(feature = "nft")]
    info_omit!(engine::remove_tap(vm));
}

#[inline(always)]
pub(crate) fn get_pre_starter(vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    engine::get_pre_starter(vm).c(d!())
}

// 执行命令
#[inline(always)]
fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;
    if res.status.success() {
        Ok(())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}

// 必须后台执行
#[inline(always)]
#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
fn cmd_exec_daemonize(cmd: &str, args: &[&str]) -> Result<()> {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => pnk!(Err(eg!(process::Command::new(cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args(args)
            .exec()))),
        Ok(_) => Ok(()),
        Err(e) => Err(e).c(d!()),
    }
}
