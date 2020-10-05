//!
//! # tt client
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

pub mod cfg_file;
pub mod cmd_line;
mod ops;

pub use cfg_file::*;
use myutil::{err::*, *};
use std::process;

fn main() {
    pnk!(cfg_file::cfg_init());
    info_omit!(
        cmd_exec("sh", &["-c", "ulimit -HSn 10240"])
            .or_else(|_| cmd_exec("sh", &["-c", "ulimit -n `ulimit -Hn`"]))
    );
    pnk!(cmd_line::parse_and_exec());
}

// 执行命令
#[inline(always)]
fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;
    if res.status.success() {
        Ok(())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}
