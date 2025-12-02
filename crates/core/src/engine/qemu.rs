//! QEMU/KVM engine implementation.
//!
//! Launches VMs via `qemu-system-x86_64` with KVM acceleration.
//! Each VM gets its own tap device connected to the host bridge.

use super::VmEngine;
use crate::model::{Vm, VmState, RUN_DIR};
use ruc::*;
use std::path::Path;
use std::process::Command;

pub struct QemuEngine;

impl Default for QemuEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QemuEngine {
    pub fn new() -> Self {
        Self
    }

    fn build_cmd(&self, vm: &Vm, image_path: &str) -> Command {
        let mut cmd = Command::new("qemu-system-x86_64");
        cmd.args(["-enable-kvm", "-daemonize"])
            .args(["-name", &vm.id])
            .args(["-m", &format!("{}M", vm.mem)])
            .args(["-smp", &vm.cpu.to_string()])
            .args([
                "-drive",
                &format!("file={image_path},format=qcow2,if=virtio"),
            ])
            .args([
                "-netdev",
                &format!(
                    "tap,id=net0,ifname=tap-{},script=no,downscript=no",
                    vm.id
                ),
            ])
            .args(["-device", "virtio-net-pci,netdev=net0"])
            .args(["-pidfile", &self.pid_path(vm)])
            .args([
                "-monitor",
                &format!("unix:{},server,nowait", self.monitor_path(vm)),
            ])
            .args(["-vnc", "none"]);
        cmd
    }

    fn pid_path(&self, vm: &Vm) -> String {
        format!("{RUN_DIR}/qemu-{}.pid", vm.id)
    }

    fn monitor_path(&self, vm: &Vm) -> String {
        format!("{RUN_DIR}/qemu-{}.sock", vm.id)
    }

    fn read_pid(&self, vm: &Vm) -> Result<u32> {
        let path = self.pid_path(vm);
        let content = std::fs::read_to_string(&path).c(d!("read pid file"))?;
        content.trim().parse::<u32>().c(d!("invalid pid"))
    }

    fn process_alive(pid: u32) -> bool {
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

impl VmEngine for QemuEngine {
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()> {
        std::fs::create_dir_all(RUN_DIR).c(d!("create runtime dir"))?;

        let output = self
            .build_cmd(vm, image_path)
            .output()
            .c(d!("spawn qemu"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("qemu launch failed: {}", stderr));
        }

        Ok(())
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        let sock = self.monitor_path(vm);
        if !Path::new(&sock).exists() {
            // Monitor socket gone means QEMU exited; re-create the VM.
            return Err(eg!(
                "VM {} has no monitor socket; it must be re-created",
                vm.id
            ));
        }

        // Send "cont" command to the QEMU monitor to resume execution
        let output = Command::new("sh")
            .args([
                "-c",
                &format!(r#"echo "cont" | socat - UNIX-CONNECT:{sock}"#),
            ])
            .output()
            .c(d!("qemu monitor cont"))?;

        if !output.status.success() {
            return Err(eg!("failed to resume VM via QEMU monitor"));
        }
        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        // Try graceful shutdown via QEMU monitor first
        let sock = self.monitor_path(vm);
        if Path::new(&sock).exists() {
            let _ = Command::new("sh")
                .args([
                    "-c",
                    &format!(r#"echo "system_powerdown" | socat - UNIX-CONNECT:{sock}"#),
                ])
                .output();

            // Give the guest a moment to shut down
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        // If still alive, send SIGTERM
        match self.read_pid(vm) {
            Ok(pid) if Self::process_alive(pid) => nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGTERM,
            )
            .c(d!("stop qemu")),
            _ => Ok(()),
        }
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        if let Ok(pid) = self.read_pid(vm)
            && Self::process_alive(pid) {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }

        let _ = std::fs::remove_file(self.pid_path(vm));
        let _ = std::fs::remove_file(self.monitor_path(vm));

        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        match self.read_pid(vm) {
            Ok(pid) if Self::process_alive(pid) => Ok(VmState::Running),
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "qemu"
    }
}
