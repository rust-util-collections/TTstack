//! VM / container engine abstraction.
//!
//! Each engine implements [`VmEngine`] to provide a uniform interface
//! for creating, starting, stopping, and destroying instances.
//!
//! Platform-specific engines:
//! - **Linux**: Qemu, Firecracker, Docker/Podman
//! - **FreeBSD**: Bhyve, Jail

#[cfg(target_os = "freebsd")]
pub mod bhyve;
#[cfg(target_os = "linux")]
pub mod docker;
#[cfg(target_os = "linux")]
pub mod firecracker;
#[cfg(target_os = "freebsd")]
pub mod jail;
#[cfg(target_os = "linux")]
pub mod qemu;

use crate::model::{Engine, Vm, VmState};
use ruc::*;

/// Trait implemented by each hypervisor / container engine.
pub trait VmEngine: Send + Sync {
    /// Create and boot a new VM from the given image path.
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()>;

    /// Start a previously stopped VM.
    fn start(&self, vm: &Vm) -> Result<()>;

    /// Gracefully stop a running VM.
    fn stop(&self, vm: &Vm) -> Result<()>;

    /// Destroy the VM and clean up all associated resources.
    fn destroy(&self, vm: &Vm) -> Result<()>;

    /// Query the current state of the VM.
    fn state(&self, vm: &Vm) -> Result<VmState>;

    /// Human-readable engine name.
    fn name(&self) -> &'static str;
}

/// Create an engine instance for the given kind.
pub fn create_engine(kind: Engine) -> Box<dyn VmEngine> {
    match kind {
        #[cfg(target_os = "linux")]
        Engine::Qemu => Box::new(qemu::QemuEngine::new()),
        #[cfg(target_os = "linux")]
        Engine::Firecracker => Box::new(firecracker::FirecrackerEngine::new()),
        #[cfg(target_os = "linux")]
        Engine::Docker => Box::new(docker::DockerEngine::new()),
        #[cfg(target_os = "freebsd")]
        Engine::Bhyve => Box::new(bhyve::BhyveEngine::new()),
        #[cfg(target_os = "freebsd")]
        Engine::Jail => Box::new(jail::JailEngine::new()),
        #[allow(unreachable_patterns)]
        other => panic!("engine {other} is not supported on this platform"),
    }
}
