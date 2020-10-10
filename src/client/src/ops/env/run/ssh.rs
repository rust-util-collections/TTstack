//!
//! A slow implementation based on SSH.
//!

use super::VmConnInfo;
use crate::SSH_VM_KEY;
use myutil::{err::*, *};
use nix::unistd::execv;
use std::ffi::CString;
use std::{
    process::{Child, Command, Stdio},
    sync::mpsc::{channel, Receiver},
    thread,
};

pub const USER: &str = "root";

/// 执行外部的 ssh 命令,
/// 收集远端的输出内容并返回之
#[allow(clippy::mutex_atomic)]
pub(super) fn exec(
    remote_cmd: &str,
    vm_conn_info: Vec<VmConnInfo>,
) -> Receiver<VmConnInfo> {
    let (s, r) = channel();

    vm_conn_info.into_iter().for_each(|mut vci| {
        let conninfo = format!("{}@{}", USER, &vci.addr);
        let port = vci.ssh_port.to_string();
        let cmd = remote_cmd.to_owned();
        let args = ["-p".to_owned(), port, conninfo, cmd].to_vec();

        let sender = s.clone();
        thread::spawn(move || {
            exec_run("ssh", args)
                .c(d!())
                .and_then(|child| {
                    child.wait_with_output().c(d!()).map(|output| {
                        vci.stdout = String::from_utf8_lossy(&output.stdout)
                            .into_owned();
                        vci.stderr = String::from_utf8_lossy(&output.stderr)
                            .into_owned();
                        vci.status_code = output.status.code().unwrap_or(255);
                    })
                })
                .unwrap_or_else(|e| {
                    vci.stderr = genlog(e);
                    vci.status_code = 255;
                });

            info_omit!(sender.send(vci));
        });
    });

    r
}

#[inline(always)]
fn exec_run(cmd: &'static str, args: Vec<String>) -> Result<Child> {
    do_exec(
        &cmd,
        args.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

#[inline(always)]
pub fn do_exec(cmd: &str, args: &[&str]) -> Result<Child> {
    Command::new(cmd)
        .args(&["-o", "ConnectTimeout=3", "-i", SSH_VM_KEY.as_str()])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .c(d!())
}

/// 串行执行 ssh 交互式操作
#[allow(clippy::mutex_atomic)]
pub(super) fn exec_interactive(vm_conn_info: Vec<VmConnInfo>) -> ! {
    pnk!(
        gen_interactive_cmds(&vm_conn_info)
            .c(d!())
            .and_then(|cmds| execv(
                &CString::new("/bin/sh").unwrap(),
                &[
                    &CString::new("/bin/sh").unwrap(),
                    &CString::new("-c").unwrap(),
                    &cmds
                ]
            )
            .c(d!()))
    );

    panic!()
}

/// 生成交互式 ssh 串行命令集合
fn gen_interactive_cmds(vm_conn_info: &[VmConnInfo]) -> Result<CString> {
    let mut res = vct![];
    vm_conn_info.iter().for_each(|vci| {
        res.push(
            format!("printf '\x1b[31;01mConnecting: {}\x1b[00m\n'; ssh -p {} -i {} -o ConnectTimeout=3 {}@{};",
                    vci.os,
                    vci.ssh_port,
                    SSH_VM_KEY.as_str(),
                    USER,
                    vci.addr
       ));
    });
    CString::new(res.join("")).c(d!())
}
