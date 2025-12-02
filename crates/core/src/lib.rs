//! TTstack core library.
//!
//! Provides shared types, engine abstractions, storage backends, and
//! network utilities used by both the host agent and central controller.
//!
//! **Supported platforms**: Linux and FreeBSD only.

// Enforce platform at compile time.
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
compile_error!("TTstack only supports Linux and FreeBSD");

pub mod api;
pub mod engine;
pub mod model;
pub mod net;
pub mod storage;
