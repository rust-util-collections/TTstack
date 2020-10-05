//!
//! # Push file to ENV
//!
//! ```shell
//! tt env push ...
//! ```
//!

mod scp;
mod ttrexec;

use super::{
    super::EnvIdRef,
    run::{get_conn_info, print_to_user, VmConnInfo},
};
use myutil::{err::*, *};
use std::{sync::mpsc::Receiver, time};

///////////////////////////////
#[derive(Default)]
pub struct EnvPush<'a> {
    pub use_ssh: bool,
    pub file_path: &'a str,
    pub env_set: Vec<&'a EnvIdRef>,
    pub time_out: u64,
    pub filter_os_prefix: Vec<&'a str>,
    pub filter_vm_id: Vec<i32>,
}
///////////////////////////////

impl<'a> EnvPush<'a> {
    /// 发送请求并打印结果
    pub fn do_req(&self) -> Result<()> {
        self.get_res().c(d!()).and_then(|(n, r)| {
            for _ in 0..n {
                r.recv_timeout(time::Duration::from_secs(self.time_out))
                    .map(print_to_user)
                    .c(d!())?;
            }
            Ok(())
        })
    }

    /// 发送请求并获取结果
    pub fn get_res(&self) -> Result<(usize, Receiver<VmConnInfo>)> {
        if "" == self.file_path {
            return Err(eg!("Empty file_path!"));
        }

        if self.env_set.is_empty() {
            return Err(eg!("Empty env_set!"));
        }

        get_conn_info(&self.env_set).c(d!()).map(|mut vci_set| {
            if !self.filter_vm_id.is_empty()
                || !self.filter_os_prefix.is_empty()
            {
                vci_set = vci_set
                    .into_iter()
                    .filter(|vci| {
                        self.filter_vm_id.iter().any(|id| vci.id == *id)
                            || self.filter_os_prefix.iter().any(|prefix| {
                                vci.os
                                    .to_lowercase()
                                    .starts_with(&prefix.to_lowercase())
                            })
                    })
                    .collect();
            }
            if self.use_ssh {
                (vci_set.len(), scp::exec(self.file_path, vci_set))
            } else {
                (vci_set.len(), ttrexec::exec(self.file_path, vci_set))
            }
        })
    }
}
