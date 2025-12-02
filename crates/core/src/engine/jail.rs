//! FreeBSD Jail engine implementation.
//!
//! Uses FreeBSD jails for lightweight OS-level virtualization.
//! Each jail gets its own root filesystem, network stack, and process space.

use super::VmEngine;
use crate::model::{Vm, VmState};
use ruc::*;
use std::process::Command;

pub struct JailEngine;

impl Default for JailEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl JailEngine {
    pub fn new() -> Self {
        Self
    }

    /// Jail name derived from VM id.
    fn jail_name(vm: &Vm) -> String {
        format!("tt-{}", vm.id)
    }
}

impl VmEngine for JailEngine {
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()> {
        let name = Self::jail_name(vm);

        // Create the jail with the given root filesystem
        let output = Command::new("jail")
            .args(["-c"])
            .arg(format!("name={name}"))
            .arg(format!("path={image_path}"))
            .arg("host.hostname=ttstack")
            .arg(format!("ip4.addr={}", vm.ip))
            .arg("persist")
            .arg("mount.devfs")
            .output()
            .c(d!("create jail"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("jail create failed: {}", stderr));
        }

        Ok(())
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        // For jails that were stopped with 'persist' flag, we re-create
        let name = Self::jail_name(vm);

        let output = Command::new("jail")
            .args(["-m"])
            .arg(format!("name={name}"))
            .arg("persist")
            .output()
            .c(d!("start jail"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("jail start failed: {}", stderr));
        }

        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        let name = Self::jail_name(vm);

        // Kill all processes then remove the jail
        let output = Command::new("jail")
            .args(["-r", &name])
            .output()
            .c(d!("stop jail"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("jail stop failed: {}", stderr));
        }

        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        // Stop first, then the image cleanup is handled by the runtime
        let _ = self.stop(vm);
        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        let name = Self::jail_name(vm);

        let output = Command::new("jls")
            .args(["-j", &name, "jid"])
            .output()
            .c(d!("query jail"))?;

        if output.status.success() {
            Ok(VmState::Running)
        } else {
            Ok(VmState::Stopped)
        }
    }

    fn name(&self) -> &'static str {
        "jail"
    }
}
