//! Bhyve engine implementation (FreeBSD only).
//!
//! Bhyve is the native hypervisor on FreeBSD. This module is only
//! compiled on FreeBSD targets via `#[cfg(target_os = "freebsd")]`.

use super::VmEngine;
use crate::model::{Vm, VmState, RUN_DIR};
use ruc::*;
use std::process::Command;

pub struct BhyveEngine;

impl BhyveEngine {
    pub fn new() -> Self {
        Self
    }
}

impl VmEngine for BhyveEngine {
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()> {
        // Load the VM into bhyve via bhyveload
        let output = Command::new("bhyveload")
            .args(["-m", &format!("{}M", vm.mem)])
            .args(["-d", image_path])
            .arg(&vm.id)
            .output()
            .c(d!("failed to run bhyveload"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("bhyveload failed: {}", stderr));
        }

        // Launch the VM
        let tap = format!("tap-{}", vm.id);
        let child = Command::new("bhyve")
            .args(["-A", "-H", "-P"])
            .args(["-c", &vm.cpu.to_string()])
            .args(["-m", &format!("{}M", vm.mem)])
            .args(["-s", "0:0,hostbridge"])
            .args(["-s", &format!("3:0,virtio-blk,{image_path}")])
            .args(["-s", &format!("4:0,virtio-net,{tap}")])
            .args(["-s", "31,lpc"])
            .args(["-l", "com1,stdio"])
            .arg(&vm.id)
            .spawn()
            .c(d!("failed to spawn bhyve"))?;

        // Record PID
        let pid_path = format!("{RUN_DIR}/bhyve-{}.pid", vm.id);
        std::fs::create_dir_all(RUN_DIR).c(d!("create pid dir"))?;
        std::fs::write(&pid_path, child.id().to_string()).c(d!("write pid"))?;

        Ok(())
    }

    fn start(&self, _vm: &Vm) -> Result<()> {
        // Bhyve doesn't support pause/resume natively;
        // "start" after destroy requires re-create.
        Err(eg!(
            "bhyve does not support in-place restart; re-create the VM"
        ))
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        let output = Command::new("bhyvectl")
            .args(["--destroy", "--vm", &vm.id])
            .output()
            .c(d!("bhyvectl destroy"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("bhyvectl failed: {}", stderr));
        }

        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        self.stop(vm)?;

        let pid_path = format!("{RUN_DIR}/bhyve-{}.pid", vm.id);
        let _ = std::fs::remove_file(&pid_path);

        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        let output = Command::new("bhyvectl")
            .args(["--get-lowmem", "--vm", &vm.id])
            .output();

        match output {
            Ok(o) if o.status.success() => Ok(VmState::Running),
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "bhyve"
    }
}
