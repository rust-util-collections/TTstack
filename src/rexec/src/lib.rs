//!
//! # fastexec
//!
//! Fast execution of remote commands and bidirectional file transfer.
//!
//! Command execution uses UDP, file transfer uses TCP.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

/// Generate log message from error
pub fn genlog<E: std::fmt::Display>(err: E) -> String {
    format!("{}", err)
}

#[cfg(feature = "client")]
pub mod client;

pub mod common;
mod sendfile;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
