//!
//! # fastexec
//!
//! 快速执行远程命令及双向转输文件.
//!
//! 命令执行使用 UDP, 传输文件使用 TCP.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

#[cfg(feature = "client")]
pub mod client;

pub mod common;
mod sendfile;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
