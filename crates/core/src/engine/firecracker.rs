//! Firecracker microVM engine implementation.
//!
//! Uses the Firecracker VMM for lightweight, fast-booting microVMs.
//! Communicates with the Firecracker process via its REST API socket.

use super::VmEngine;
use crate::model::{RUN_DIR, Vm, VmState};
use ruc::*;
use std::path::Path;
use std::process::Command;

pub struct FirecrackerEngine;

impl Default for FirecrackerEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FirecrackerEngine {
    pub fn new() -> Self {
        Self
    }

    fn socket_path(vm: &Vm) -> String {
        format!("{RUN_DIR}/fc-{}.sock", vm.id)
    }

    fn pid_path(vm: &Vm) -> String {
        format!("{RUN_DIR}/fc-{}.pid", vm.id)
    }

    fn config_path(vm: &Vm) -> String {
        format!("{RUN_DIR}/fc-{}.json", vm.id)
    }

    fn read_pid(vm: &Vm) -> Result<u32> {
        let path = Self::pid_path(vm);
        let content = std::fs::read_to_string(&path).c(d!("read fc pid"))?;
        content.trim().parse::<u32>().c(d!("invalid pid"))
    }

    fn write_config(&self, vm: &Vm, image_path: &str) -> Result<()> {
        let tap = crate::net::tap_name(&vm.id);
        let config = serde_json::json!({
            "boot-source": {
                "kernel_image_path": format!("{image_path}/vmlinux"),
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
            },
            "drives": [{
                "drive_id": "rootfs",
                "path_on_host": format!("{image_path}/rootfs.ext4"),
                "is_root_device": true,
                "is_read_only": false
            }],
            "machine-config": {
                "vcpu_count": vm.cpu,
                "mem_size_mib": vm.mem,
            },
            "network-interfaces": [{
                "iface_id": "eth0",
                "host_dev_name": tap,
            }]
        });

        let path = Self::config_path(vm);
        let json = serde_json::to_string_pretty(&config).c(d!("serialize config"))?;
        std::fs::write(&path, json).c(d!("write config"))
    }
}

impl VmEngine for FirecrackerEngine {
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()> {
        std::fs::create_dir_all(RUN_DIR).c(d!("create runtime dir"))?;

        self.write_config(vm, image_path)?;

        let sock = Self::socket_path(vm);
        let config = Self::config_path(vm);

        let mut child = Command::new("firecracker")
            .args(["--api-sock", &sock])
            .args(["--config-file", &config])
            .spawn()
            .c(d!("spawn firecracker"))?;

        let pid = child.id();
        std::fs::write(Self::pid_path(vm), pid.to_string()).c(d!("write pid"))?;

        // Spawn a reaper thread so the child process is wait()ed on,
        // preventing zombie processes if the Firecracker VM exits.
        std::thread::spawn(move || {
            let _ = child.wait();
        });

        Ok(())
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        let sock = Self::socket_path(vm);
        if !Path::new(&sock).exists() {
            return Err(eg!("firecracker socket not found for VM {}", vm.id));
        }

        // Resume a paused VM via the Firecracker API
        let output = Command::new("curl")
            .args([
                "--unix-socket",
                &sock,
                "-s",
                "-X",
                "PATCH",
                "http://localhost/vm",
                "-H",
                "Content-Type: application/json",
                "-d",
                r#"{"state": "Resumed"}"#,
            ])
            .output()
            .c(d!("resume firecracker"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("firecracker resume failed: {}", stderr));
        }
        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        let sock = Self::socket_path(vm);
        if !Path::new(&sock).exists() {
            return Ok(());
        }

        // Pause the VM via the Firecracker API (preserves the process)
        let output = Command::new("curl")
            .args([
                "--unix-socket",
                &sock,
                "-s",
                "-X",
                "PATCH",
                "http://localhost/vm",
                "-H",
                "Content-Type: application/json",
                "-d",
                r#"{"state": "Paused"}"#,
            ])
            .output()
            .c(d!("pause firecracker"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("firecracker pause failed: {}", stderr));
        }
        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        if let Ok(pid) = Self::read_pid(vm) {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }

        let _ = std::fs::remove_file(Self::socket_path(vm));
        let _ = std::fs::remove_file(Self::pid_path(vm));
        let _ = std::fs::remove_file(Self::config_path(vm));

        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        match Self::read_pid(vm) {
            Ok(pid) if Path::new(&format!("/proc/{pid}")).exists() => {
                // Query Firecracker API to distinguish Running vs Paused
                let sock = Self::socket_path(vm);
                if Path::new(&sock).exists() {
                    if let Ok(output) = Command::new("curl")
                        .args([
                            "--unix-socket", &sock,
                            "-s",
                            "http://localhost/vm",
                        ])
                        .output()
                    {
                        let body = String::from_utf8_lossy(&output.stdout);
                        if body.contains("\"Paused\"") {
                            return Ok(VmState::Paused);
                        }
                    }
                }
                Ok(VmState::Running)
            }
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "firecracker"
    }
}
