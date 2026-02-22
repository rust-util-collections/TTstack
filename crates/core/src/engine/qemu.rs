//! QEMU/KVM engine implementation.
//!
//! Launches VMs via `qemu-system-x86_64` with KVM acceleration.
//! Each VM gets its own tap device connected to the host bridge.

use super::VmEngine;
use crate::model::{RUN_DIR, Vm, VmState};
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

    /// Resolve the disk image file path.
    ///
    /// If `image_path` is a regular file, use it directly.
    /// If it's a directory (e.g. a ZFS dataset or Btrfs subvolume),
    /// look for a qcow2 file inside it.
    fn resolve_disk(image_path: &str) -> String {
        use std::path::Path;
        let p = Path::new(image_path);
        if p.is_dir() {
            // Look for a .qcow2 file first, then any single file
            if let Ok(entries) = std::fs::read_dir(p) {
                let files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .collect();
                if let Some(qcow2) = files
                    .iter()
                    .find(|f| f.path().extension().is_some_and(|ext| ext == "qcow2"))
                {
                    return qcow2.path().to_string_lossy().into_owned();
                }
                if files.len() == 1 {
                    return files[0].path().to_string_lossy().into_owned();
                }
            }
            // Fallback: assume disk.qcow2
            format!("{image_path}/disk.qcow2")
        } else {
            image_path.to_string()
        }
    }

    fn build_cmd(&self, vm: &Vm, image_path: &str) -> Command {
        let tap = crate::net::tap_name(&vm.id);
        let disk = Self::resolve_disk(image_path);
        let mut cmd = Command::new("qemu-system-x86_64");
        cmd.args(["-enable-kvm", "-daemonize"])
            .args(["-name", &vm.id])
            .args(["-m", &format!("{}M", vm.mem)])
            .args(["-smp", &vm.cpu.to_string()])
            .args(["-drive", &format!("file={disk},format=qcow2,if=virtio")])
            .args([
                "-netdev",
                &format!("tap,id=net0,ifname={tap},script=no,downscript=no"),
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
        // Pause the VM via QEMU monitor "stop" command.
        // This freezes the vCPU without killing the QEMU process,
        // allowing later resume via "cont".
        let sock = self.monitor_path(vm);
        if Path::new(&sock).exists() {
            let _ = Command::new("sh")
                .args([
                    "-c",
                    &format!(r#"echo "stop" | socat - UNIX-CONNECT:{sock}"#),
                ])
                .output();
        }
        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        if let Ok(pid) = self.read_pid(vm)
            && Self::process_alive(pid)
        {
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
            Ok(pid) if Self::process_alive(pid) => {
                // Query QEMU monitor to distinguish Running vs Paused
                let sock = self.monitor_path(vm);
                if Path::new(&sock).exists()
                    && let Ok(output) = Command::new("sh")
                        .args([
                            "-c",
                            &format!(r#"echo "info status" | socat - UNIX-CONNECT:{sock}"#),
                        ])
                        .output()
                {
                    let body = String::from_utf8_lossy(&output.stdout);
                    if body.contains("paused") {
                        return Ok(VmState::Paused);
                    }
                }
                Ok(VmState::Running)
            }
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "qemu"
    }
}
