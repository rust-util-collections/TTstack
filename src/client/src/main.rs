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
    info_omit!(set_nofile());
    pnk!(cmd_line::parse_and_exec());
}

// avoie "too many files open"
fn set_nofile() -> Result<()> {
    let hard_limit = cmd_exec_with_output("ulimit", &["-H", "-n"])
        .c(d!())?
        .parse::<u64>()
        .unwrap_or(10240)
        .to_string();
    cmd_exec("ulimit", &["-S", "-n", &hard_limit]).c(d!())
}

#[inline(always)]
fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    cmd_exec_with_output(cmd, args).c(d!()).map(|_| ())
}

#[inline(always)]
fn cmd_exec_with_output(cmd: &str, args: &[&str]) -> Result<String> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;
    if res.status.success() {
        Ok(String::from_utf8_lossy(&res.stdout).into_owned())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}
