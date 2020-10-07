//!
//! # NAT mocker
//!

use crate::Vm;
use myutil::err::*;

pub(crate) fn set_rule(_vm: &Vm) -> Result<()> {
    Ok(())
}

pub(crate) fn clean_rule(_vm_set: &[&Vm]) -> Result<()> {
    Ok(())
}

pub(crate) fn deny_outgoing(_vm_set: &[&Vm]) -> Result<()> {
    Ok(())
}

pub(crate) fn allow_outgoing(_vm_set: &[&Vm]) -> Result<()> {
    Ok(())
}
