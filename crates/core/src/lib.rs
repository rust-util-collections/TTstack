//! TTstack core library.
//!
//! Provides shared types, engine abstractions, storage backends, and
//! network utilities used by both the host agent and central controller.
//!
//! The [`api`] and [`model`] modules are platform-independent and used
//! by all components (CLI, controller, agent).
//!
//! The [`engine`], [`net`], and [`storage`] modules are only available
//! on Linux and FreeBSD where the agent daemon runs.

pub mod api;
pub mod model;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod engine;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod net;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod storage;
