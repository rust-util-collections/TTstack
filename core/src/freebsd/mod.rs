//!
//! # tt, Temporary Test.
//!
//! core 模块实现服务端的核心逻辑.
//!
//! Use Bhyve + IPFW + ZFS on FreeBSD.
//!

pub(crate) mod nat;
pub(crate) mod vm;

use crate::{ImagePath, OsName, Vm, VmId, VmKind};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};

const CLONE_MARK: &str = "clone_";

lazy_static! {
    static ref ZFS_ROOT: &'static str = pnk!(imgroot_register(None));
}

fn imgroot_register(imgpath: Option<&str>) -> Option<&'static str> {
    static mut ROOT: Option<String> = None;
    if let Some(path) = imgpath {
        unsafe {
            ROOT.replace(
                path.trim_start_matches("/dev/zvol/")
                    .trim_end_matches('/')
                    .to_owned(),
            );
        }
    }

    unsafe { ROOT.as_deref() }
}

/////////////////
// Entry Point //
/////////////////

/// 全局入口, 必须首先调用
#[inline(always)]
pub fn exec(
    imgpath: &str,
    cb: fn() -> Result<()>,
    serv_ip: &str,
) -> Result<()> {
    imgroot_register(Some(imgpath));
    EntryPoint::new().exec(cb, serv_ip).c(d!())
}

struct EntryPoint;

impl EntryPoint {
    fn new() -> Self {
        EntryPoint
    }

    fn exec(self, cb: fn() -> Result<()>, serv_ip: &str) -> Result<()> {
        nat::init(serv_ip)
            .c(d!())
            .and_then(|_| vm::init().c(d!()))
            .and_then(|_| cb().c(d!()))
    }
}

impl Drop for EntryPoint {
    fn drop(&mut self) {
        info_omit!(vm::env_clean());
    }
}

//////////////////
// Support List //
//////////////////

/// 获取服务端支持的系统列表和对应的 Vm 镜像路径,
/// 排除基础快照、镜像内部分区、Clone 临时镜像三类对象
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
                        || is_partition(os)
                        || os.starts_with(CLONE_MARK))
            })
            .map(|(os, i)| {
                (os.to_lowercase(), i.to_string_lossy().into_owned())
            })
            .collect()
    })
}

// the name-rule of partition is
// diffrent for Linux_OS and FreeBSD_OS,
//
// eg:
// - CentOS7.5s3
// - FreeBSD12.1p2
fn is_partition(obj: &str) -> bool {
    let s = obj.trim_end_matches(char::is_numeric);
    s.ends_with('s') || s.ends_with('p')
}

// 读取 ImagePath 下的所有 zfs volume
#[inline(always)]
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

/// stop an env
#[inline(always)]
pub fn pause(id: VmId) -> Result<()> {
    vm::bhyve_stop(id).c(d!())
}

/// restart an env
#[inline(always)]
pub fn resume(vm: &Vm) -> Result<()> {
    vm::bhyve_exec(vm).c(d!())
}

/// 根据镜像前缀识别虚拟机引擎
pub fn vm_kind(os: &str) -> Result<VmKind> {
    let os = os.to_lowercase();
    for (prefix, kind) in &[("bhyve:", VmKind::Bhyve)] {
        if os.starts_with(prefix) {
            return Ok(*kind);
        }
    }
    Err(eg!(
        "Invalid OS name, it should starts with one of [ bhvye ]."
    ))
}
