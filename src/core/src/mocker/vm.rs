//!
//! # Mocker for Virtual Machine Mgmt
//!

use crate::Vm;
use myutil::err::*;

pub(crate) fn start(_: &Vm) -> Result<()> {
    Ok(())
}

// Do nothing on freebsd.
pub(crate) fn zobmie_clean() {}

pub(crate) fn post_clean(_: &Vm) {}

pub(crate) fn get_pre_starter(_vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    Ok(pre_start)
}

fn pre_start(_vm: &Vm) -> Result<()> {
    Ok(())
}
