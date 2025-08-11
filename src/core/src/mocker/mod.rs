#![warn(unused_import_braces, unused_extern_crates)]
#![allow(missing_docs)]

pub(crate) mod nat;
pub(crate) mod vm;

use crate::{ImagePath, OsName, Vm, VmId, VmKind};
use ruc::*;
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
    use std::fs;
    
    let res = map! {
        "centos7.0".to_string() => format!("{}/centos7.{}", img_path, 0),
        "centos7.1".to_string() => format!("{}/centos7.{}", img_path, 1),
        "centos7.2".to_string() => format!("{}/centos7.{}", img_path, 2),
        "centos7.3".to_string() => format!("{}/centos7.{}", img_path, 3),
        "ubuntu20.04".to_string() => format!("{}/ubuntu20.04", img_path),
        "ubuntu22.04".to_string() => format!("{}/ubuntu22.04", img_path),
        "qemu:centos7.0".to_string() => format!("{}/qemu:centos7.{}", img_path, 0),
        "qemu:centos7.1".to_string() => format!("{}/qemu:centos7.{}", img_path, 1),
        "qemu:ubuntu20.04".to_string() => format!("{}/qemu:ubuntu20.04", img_path),
        "qemu:ubuntu22.04".to_string() => format!("{}/qemu:ubuntu22.04", img_path),
    };

    // Create empty mock files for testing
    for path in res.values() {
        info_omit!(fs::File::create(path));
    }

    Ok(res)
}

#[inline(always)]
pub fn pause(_id: VmId) -> Result<()> {
    Ok(())
}

#[inline(always)]
pub fn resume(_vm: &Vm) -> Result<()> {
    Ok(())
}

pub fn vm_kind(os: &str) -> Result<VmKind> {
    let os = os.to_lowercase();
    if os.starts_with("qemu:") {
        Ok(VmKind::Qemu)
    } else if os.starts_with("fire:") {
        Ok(VmKind::FireCracker) 
    } else {
        Ok(VmKind::Qemu) // Default to Qemu for compatibility
    }
}
