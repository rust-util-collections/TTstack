//!
//! # Cgroup2
//!
//! 每个 Vm 分配一个独立的 Cgroup, 通达 Drop Trait 自动清理资源.
//!

use crate::{asleep, linux::vm::util, VmId, POOL};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use nix::{
    sys::signal::{self, kill},
    unistd::Pid,
};
use std::{fs, io::Write, path::PathBuf, process};

// 采用 private mount, 多服务可使用同一挂载点
const CGROUP_ROOT_PATH: &str = "/tmp/.ttcgroup";

// 管理进程的 Cgroup 路径
const CGROUP_ADMIN_PATH: &str = "/tmp/.ttcgroup/ttadmin";

// 重置管理进程的 Cgroup 状态
fn cgrp_reset_admin() -> Result<()> {
    lazy_static! {
        static ref CGRP_PROCS: String =
            format!("{}/cgroup.procs", CGROUP_ADMIN_PATH);
    }

    fs::OpenOptions::new()
        .append(true)
        .open(CGRP_PROCS.as_str())
        .c(d!())?
        .write(process::id().to_string().as_bytes())
        .c(d!())
        .map(|_| ())
}

/// 挂载 Cgroup2 根目录结构,
/// **MUST** do `mount` first! (before CGROUP_ADMIN_PATH)
pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(CGROUP_ROOT_PATH)
        .c(d!())
        .and_then(|_| util::mount_cgroup2(CGROUP_ROOT_PATH).c(d!()))
        .and_then(|_| fs::create_dir_all(CGROUP_ADMIN_PATH).c(d!()))
}

// 确认 Cgroup 根路径已挂载
fn cgroup2_ready() -> bool {
    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push("cgroup.procs");
    path.is_file()
}

/// 按 '/VmId' 分配挂载点
pub(in crate::linux) fn alloc_mnt_point(id: VmId) -> Result<PathBuf> {
    if !cgroup2_ready() {
        return Err(eg!("The fucking world is over!"));
    }

    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push(id.to_string());
    if !path.exists() {
        fs::create_dir(&path).c(d!()).map(|_| path)
    } else if 0 == get_proc_meta_path(id).c(d!())?.metadata().c(d!())?.len() {
        // cgroup.procs 文件为空, 即:
        // 挂载点已存在, 但没有进程参与其中
        Ok(path)
    } else {
        Err(eg!("The fucking world is over!"))
    }
}

/// 将 vm 进程加入到指定的 Cgroup 中
#[inline(always)]
pub(in crate::linux) fn add_vm(id: VmId, pid: crate::Pid) -> Result<()> {
    add_proc(id, pid).c(d!())
}

// 将指定进程加入到指定的 Cgroup 中
fn add_proc(id: VmId, pid: crate::Pid) -> Result<()> {
    get_proc_meta_path(id)
        .c(d!())
        .and_then(|meta| {
            fs::OpenOptions::new().append(true).open(meta).c(d!())
        })
        .and_then(|mut f| {
            f.write(pid.to_string().as_bytes()).c(d!()).map(|_| ())
        })
}

/// 清理 Vm 进程[组]
pub(crate) fn kill_vm(id: VmId) -> Result<()> {
    get_proc_meta_path(id)
        .c(d!())
        .and_then(|p| kill_cgrp(p).c(d!()))
}

// 对指定 Cgroup 下的所有进程, 执行 `kill -9`
fn kill_cgrp(cgpath: PathBuf) -> Result<()> {
    fs::read(&cgpath)
        .c(d!())
        .and_then(|b| String::from_utf8(b).c(d!()))
        .and_then(|s| {
            let mut failed_list = vct![];
            s.lines().for_each(|pid| {
                let pid = pnk!(pid.parse::<u32>());
                // 启动 Vm 进程之前, 控制进程会先进入对应的 Cgroup
                if process::id() == pid {
                    info_omit!(cgrp_reset_admin());
                    return;
                }
                kill(Pid::from_raw(pid as libc::pid_t), signal::SIGTERM)
                    .c(d!())
                    .unwrap_or_else(|e| failed_list.push((pid, e)));
            });
            alt!(
                failed_list.is_empty(),
                Ok(()),
                Err(eg!(format!("{:#?}", failed_list)))
            )
        })
        .and_then(|_| cgpath.parent().ok_or(eg!("shit!")))
        .map(|dir| {
            let dir = dir.to_owned();
            POOL.spawn_ok(async move {
                asleep(5).await;
                info_omit!(fs::remove_dir(&dir));
            })
        })
}

// 获取 Cgroup2 中存放PID列表的文件路径
#[inline(always)]
fn get_proc_meta_path(id: VmId) -> Result<PathBuf> {
    let mut mnt = get_mnt_point(id).c(d!())?;
    mnt.push("cgroup.procs");
    alt!(mnt.is_file(), Ok(mnt), Err(eg!()))
}

// 获取指定 Vm 的 Cgroup 挂载点
fn get_mnt_point(id: VmId) -> Result<PathBuf> {
    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push(id.to_string());

    if path.exists() && path.is_dir() {
        Ok(path)
    } else {
        Err(eg!("Not exists!"))
    }
}
