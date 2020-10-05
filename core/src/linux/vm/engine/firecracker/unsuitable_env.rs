use crate::linux::vm::Vm;
use myutil::{err::*, *};

pub(crate) const LOG_DIR: &str = "";

pub(crate) fn start(_vm: &Vm) -> Result<()> {
    Err(eg!("Unsuitable environment!"))
}

pub(crate) fn pre_starter(_vm: &Vm) -> Result<()> {
    Err(eg!("Unsuitable environment!"))
}

pub(crate) fn remove_image(_vm: &Vm) -> Result<()> {
    Err(eg!("Unsuitable environment!"))
}

#[cfg(feature = "nft")]
pub(crate) fn remove_tap(_vm: &Vm) -> Result<()> {
    Err(eg!("Unsuitable environment!"))
}
