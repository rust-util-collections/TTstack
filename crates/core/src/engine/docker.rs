//! Docker / Podman container engine implementation.
//!
//! Auto-detects whether `docker` or `podman` is available and uses
//! whichever is found (preferring podman for rootless operation).

use super::VmEngine;
use crate::model::{Vm, VmState};
use ruc::*;
use std::process::Command;
use std::sync::LazyLock;

/// Cached path to the container runtime binary.
static RUNTIME: LazyLock<&'static str> = LazyLock::new(|| {
    if Command::new("podman").arg("--version").output().is_ok() {
        "podman"
    } else {
        "docker"
    }
});

pub struct DockerEngine;

impl Default for DockerEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerEngine {
    pub fn new() -> Self {
        Self
    }

    fn runtime() -> &'static str {
        &RUNTIME
    }

    /// Container name derived from VM id.
    fn container_name(vm: &Vm) -> String {
        format!("tt-{}", vm.id)
    }
}

impl VmEngine for DockerEngine {
    fn create(&self, vm: &Vm, _image_path: &str) -> Result<()> {
        let name = Self::container_name(vm);
        let rt = Self::runtime();

        let mut cmd = Command::new(rt);
        cmd.args(["run", "-d", "--name", &name])
            .args(["--cpus", &vm.cpu.to_string()])
            .args(["--memory", &format!("{}m", vm.mem)]);

        // Publish port mappings
        for (&guest, &host) in &vm.port_map {
            cmd.args(["-p", &format!("{host}:{guest}")]);
        }

        // The image name is used directly as the container image reference
        cmd.arg(&vm.image);

        let output = cmd.output().c(d!("failed to spawn container"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("{} run failed: {}", rt, stderr));
        }

        Ok(())
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        let name = Self::container_name(vm);
        let output = Command::new(Self::runtime())
            .args(["start", &name])
            .output()
            .c(d!())?;

        alt!(
            output.status.success(),
            return Err(eg!("container start failed"))
        );
        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        let name = Self::container_name(vm);
        let output = Command::new(Self::runtime())
            .args(["stop", "-t", "10", &name])
            .output()
            .c(d!())?;

        alt!(
            output.status.success(),
            return Err(eg!("container stop failed"))
        );
        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        let name = Self::container_name(vm);
        // Force remove the container
        let output = Command::new(Self::runtime())
            .args(["rm", "-f", &name])
            .output()
            .c(d!())?;

        alt!(
            output.status.success(),
            return Err(eg!("container remove failed"))
        );
        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        let name = Self::container_name(vm);
        let output = Command::new(Self::runtime())
            .args(["inspect", "-f", "{{.State.Status}}", &name])
            .output()
            .c(d!())?;

        if !output.status.success() {
            return Ok(VmState::Stopped);
        }

        let status = String::from_utf8_lossy(&output.stdout);
        match status.trim() {
            "running" => Ok(VmState::Running),
            "exited" | "dead" | "created" => Ok(VmState::Stopped),
            _ => Ok(VmState::Failed),
        }
    }

    fn name(&self) -> &'static str {
        "docker"
    }
}
