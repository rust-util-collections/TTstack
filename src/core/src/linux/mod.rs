//!
//! # tt, Temporary Test.
//!
//! core 模块实现服务端的核心逻辑.
//!
//! use Qemu\FireCracker + Nftables on Linux.
//!

pub(crate) mod nat;
pub(crate) mod vm;

#[cfg(feature = "zfs")]
use crate::{imgroot_register, CLONE_MARK};
use crate::{ImagePath, OsName, Vm, VmId, VmKind};
use myutil::{err::*, *};
use nix::sched::{clone, CloneFlags};
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};

/////////////////
// Entry Point //
/////////////////

/// 全局入口, 必须首先调用
#[cfg(feature = "zfs")]
#[inline(always)]
pub fn exec(
    imgpath: &str,
    cb: fn() -> Result<()>,
    serv_ip: &str,
) -> Result<()> {
    imgroot_register(Some(imgpath));
    do_exec(cb, serv_ip).c(d!())
}

/// 全局入口, 必须首先调用
#[cfg(not(feature = "zfs"))]
#[inline(always)]
pub fn exec(
    _imgpath: &str,
    cb: fn() -> Result<()>,
    serv_ip: &str,
) -> Result<()> {
    do_exec(cb, serv_ip).c(d!())
}

fn do_exec(cb: fn() -> Result<()>, serv_ip: &str) -> Result<()> {
    const STACK_SIZ: usize = 1024 * 1024;
    let mut stack = Vec::with_capacity(STACK_SIZ);
    unsafe {
        stack.set_len(STACK_SIZ);
    }

    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWPID);

    let ops = || -> isize {
        info!(
            vm::util::mount_make_rprivate()
                .c(d!())
                .and_then(|_| vm::util::mount_dynfs_proc().c(d!()))
                .and_then(|_| vm::util::mount_tmp_tmpfs().c(d!()))
                .and_then(|_| vm::engine::init().c(d!()))
                .and_then(|_| vm::cgroup::init().c(d!()))
                .and_then(|_| nat::init(serv_ip).c(d!()))
                .and_then(|_| cb().c(d!()))
        )
        .and(Ok(0))
        .or::<Result<i32>>(Ok(-1))
        .unwrap()
    };

    clone(
        Box::new(ops),
        stack.as_mut_slice(),
        flags,
        Some(libc::SIGCHLD),
    )
    .c(d!())
    .map(|_| ())
}

//////////////////
// Support List //
//////////////////

/// 获取服务端支持的系统列表和对应的 Vm 镜像路径,
/// 排除基础快照、镜像内部分区、Clone 临时镜像三类对象
#[cfg(feature = "zfs")]
pub fn get_os_info(img_path: &str) -> Result<HashMap<OsName, ImagePath>> {
    get_image_path(img_path).c(d!()).map(|path| {
        path.iter()
            .filter_map(|i| {
                i.file_name()
                    .map(|j| j.to_str())
                    .flatten()
                    .map(|os| (os, i))
            })
            .filter(|(os, _)| {
                vm_kind(os).is_ok()
                    && !(os.contains('@')
                        || os.contains("-part")
                        || os.starts_with(CLONE_MARK))
            })
            .map(|(os, i)| {
                (os.to_lowercase(), i.to_string_lossy().into_owned())
            })
            .collect()
    })
}

/// 获取服务端支持的系统列表和对应的 Vm 镜像路径
#[cfg(not(feature = "zfs"))]
pub fn get_os_info(img_path: &str) -> Result<HashMap<OsName, ImagePath>> {
    get_image_path(img_path).c(d!()).map(|path| {
        path.iter()
            .filter_map(|i| {
                i.file_name()
                    .map(|j| j.to_str())
                    .flatten()
                    .map(|os| (os, i))
            })
            .filter(|(os, _)| vm_kind(os).is_ok())
            .map(|(os, i)| {
                (os.to_lowercase(), i.to_string_lossy().into_owned())
            })
            .collect()
    })
}

/// 读取 zfs snapshot 集合
#[cfg(feature = "zfs")]
fn get_image_path(img_path: &str) -> Result<Vec<PathBuf>> {
    let mut res = vct![];
    let dir = Path::new(img_path);
    if dir.is_dir() {
        for entry in fs::read_dir(dir).c(d!())? {
            let entry = entry.c(d!())?;
            let path = entry.path();
            if let Some(p) = path.to_str() {
                res.push(PathBuf::from(p));
            }
        }
    }
    Ok(res)
}

/// 递归读取 ImagePath 下的所有以 ".qemu" 结尾的文件
#[inline(always)]
#[cfg(not(feature = "zfs"))]
fn get_image_path(img_path: &str) -> Result<Vec<PathBuf>> {
    walk_gen(&Path::new(img_path)).c(d!())
}

// recursive function
#[inline(always)]
#[cfg(not(feature = "zfs"))]
fn walk_gen(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut res = vct![];

    if dir.is_dir() {
        for entry in fs::read_dir(dir).c(d!())? {
            let entry = entry.c(d!())?;
            let path = entry.path();
            if path.is_dir() {
                res.append(&mut walk_gen(&path).c(d!())?);
            } else if let Some(p) = path.to_str() {
                res.push(PathBuf::from(p));
            }
        }
    }

    Ok(res)
}

/// stop an env
#[inline(always)]
pub fn pause(id: VmId) -> Result<()> {
    vm::cgroup::kill_vm(id).c(d!())
}

/// restart an env
#[inline(always)]
pub fn resume(vm: &Vm) -> Result<()> {
    vm::start(vm).c(d!())
}

/// 根据镜像前缀识别虚拟机引擎
pub fn vm_kind(os: &str) -> Result<VmKind> {
    let os = os.to_lowercase();
    for (prefix, kind) in
        &[("qemu:", VmKind::Qemu), ("fire:", VmKind::FireCracker)]
    {
        if os.starts_with(prefix) {
            return Ok(*kind);
        }
    }
    Err(eg!(
        "Invalid OS name, it should starts with one of [ qemu, fire[cracker] ]."
    ))
}
