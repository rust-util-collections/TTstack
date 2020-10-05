mod firecracker;
mod qemu;

use crate::{linux::CLONE_MARK, Vm, VmKind};
use myutil::{err::*, *};
use std::{fs, path::PathBuf};

// TODO: support more vm-engine
#[inline(always)]
pub(super) fn start(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::start(vm).c(d!()),
        VmKind::FireCracker => firecracker::start(vm).c(d!()),
        _ => Err(eg!("Unsupported VmKind!")),
    }
}

#[inline(always)]
pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(firecracker::LOG_DIR)
        .c(d!())
        .and_then(|_| {
            // firecracker also need this!
            qemu::init().c(d!())
        })
}

#[inline(always)]
pub(super) fn get_pre_starter(vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    match vm.kind {
        VmKind::Qemu => Ok(qemu::pre_starter),
        VmKind::FireCracker => Ok(firecracker::pre_starter),
        _ => Err(eg!("Unsupported VmKind!")),
    }
}

#[inline(always)]
pub(super) fn remove_image(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::remove_image(vm).c(d!()),
        VmKind::FireCracker => firecracker::remove_image(vm).c(d!()),
        _ => Err(eg!("The fucking world is over!")),
    }
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn remove_tap(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::remove_tap(vm).c(d!()),
        VmKind::FireCracker => firecracker::remove_tap(vm).c(d!()),
        _ => Err(eg!("The fucking world is over!")),
    }
}

// 命名格式为: ${CLONE_MARK}_VmId
#[inline(always)]
fn vmimg_path(vm: &Vm) -> PathBuf {
    let mut vmimg_path = vm.image_path.clone();
    let vmimg_name = format!("{}{}", CLONE_MARK, vm.id);
    vmimg_path.set_file_name(vmimg_name);
    vmimg_path
}
