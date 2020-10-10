//!
//! A fast implementation based on ttrexec.
//!

use super::{ssh::USER, VmConnInfo};
use myutil::{err::*, *};
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};
use ttrexec::client::req_exec;

pub(crate) fn exec(
    remote_cmd: &str,
    vm_conn_info: Vec<VmConnInfo>,
) -> Receiver<VmConnInfo> {
    let (s, r) = channel();
    vm_conn_info.into_iter().for_each(|mut vci| {
        // keep the same-workdir with SSH
        let cmd = format!("cd ~{};{}", USER, remote_cmd);
        let sender = s.clone();
        thread::spawn(move || {
            match req_exec(&format!("{}:{}", vci.addr, vci.ttrexec_port), &cmd)
            {
                Ok(resp) => {
                    vci.stdout = resp.stdout.into_owned();
                    vci.stderr = resp.stderr.into_owned();
                    vci.status_code = resp.code;
                }
                Err(e) => {
                    vci.stderr = genlog(e);
                    vci.status_code = 255;
                }
            }
            info_omit!(sender.send(vci));
        });
    });

    r
}
