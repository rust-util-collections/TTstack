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
        let config = serde_json::json!({
            "boot-source": {
                "kernel_image_path": format!("{image_path}/vmlinux"),
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
            },
            "drives": [{
                "drive_id": "rootfs",
                "path_on_host": image_path,
                "is_root_device": true,
                "is_read_only": false
            }],
            "machine-config": {
                "vcpu_count": vm.cpu,
                "mem_size_mib": vm.mem,
            },
            "network-interfaces": [{
                "iface_id": "eth0",
                "host_dev_name": format!("tap-{}", vm.id),
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

        let child = Command::new("firecracker")
            .args(["--api-sock", &sock])
            .args(["--config-file", &config])
            .spawn()
            .c(d!("spawn firecracker"))?;

        std::fs::write(Self::pid_path(vm), child.id().to_string()).c(d!("write pid"))
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        let sock = Self::socket_path(vm);
        if !Path::new(&sock).exists() {
            return Err(eg!("firecracker socket not found for VM {}", vm.id));
        }

        let output = Command::new("curl")
            .args([
                "--unix-socket",
                &sock,
                "-X",
                "PUT",
                "http://localhost/actions",
                "-H",
                "Content-Type: application/json",
                "-d",
                r#"{"action_type": "InstanceStart"}"#,
            ])
            .output()
            .c(d!("start firecracker"))?;

        if !output.status.success() {
            return Err(eg!("firecracker start failed"));
        }
        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        if let Ok(pid) = Self::read_pid(vm) {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGTERM,
            );
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
            Ok(pid) if Path::new(&format!("/proc/{pid}")).exists() => Ok(VmState::Running),
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "firecracker"
    }
}
