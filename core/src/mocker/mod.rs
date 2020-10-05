#![warn(unused_import_braces, unused_extern_crates)]
#![allow(missing_docs)]

pub(crate) mod nat;
pub(crate) mod vm;

use crate::{ImagePath, OsName, Vm, VmId, VmKind};
use myutil::{err::*, *};
use std::collections::HashMap;

/////////////////
// Entry Point //
/////////////////
pub fn exec(
    _imgpath: &str,
    cb: fn() -> Result<()>,
    _serv_ip: &str,
) -> Result<()> {
    cb().c(d!())
}

//////////////////
// Support List //
//////////////////

pub fn get_os_info(img_path: &str) -> Result<HashMap<OsName, ImagePath>> {
    super::test::get_os_info(img_path).c(d!())
}

#[inline(always)]
pub fn pause(_id: VmId) -> Result<()> {
    Ok(())
}

#[inline(always)]
pub fn resume(_vm: &Vm) -> Result<()> {
    Ok(())
}

pub fn vm_kind(_os: &str) -> Result<VmKind> {
    Ok(VmKind::Unknown)
}
