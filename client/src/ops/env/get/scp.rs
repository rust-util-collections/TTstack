//!
//! A slow implementation based on SSH.
//!

use super::super::{
    run::{ssh, VmConnInfo},
    POOL,
};
use myutil::{err::*, *};
use std::sync::mpsc::{channel, Receiver};

/// 执行外部的 scp 命令,
/// 收集远端的输出内容并返回之
pub(super) fn exec(
    file_path: &str,
    vm_conn_info: Vec<VmConnInfo>,
) -> Receiver<VmConnInfo> {
    let (s, r) = channel();

    vm_conn_info
        .into_iter()
        .filter_map(|vci| {
            let remote_path =
                format!("{}@{}:{}", ssh::USER, &vci.addr, file_path);
            let port = vci.ssh_port.to_string();
            let local_file = format!(
                "{}{{{}#{}#{}}}",
                file_path.rsplitn(2, '/').next().unwrap(),
                &vci.os,
                vci.addr,
                vci.ssh_port,
            );
            let args = &["-P", &port, &remote_path, &local_file];
            info!(ssh::do_exec("scp", args).c(d!()).map(|child| (child, vci)))
                .ok()
        })
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|(child, mut vci)| {
            let sender = s.clone();
            POOL.spawn(move || {
                child
                    .wait_with_output()
                    .c(d!())
                    .map(|output| {
                        vci.stdout = String::from_utf8_lossy(&output.stdout)
                            .into_owned();
                        vci.stderr = String::from_utf8_lossy(&output.stderr)
                            .into_owned();
                    })
                    .unwrap_or_else(|e| {
                        vci.stderr = genlog(e);
                    });

                info_omit!(sender.send(vci));
            });
        });

    r
}
