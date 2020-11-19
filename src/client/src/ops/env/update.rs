//!
//! # Env
//!
//! ```shell
//! tt env update --kick-vm=[OS_PREFIX] --update-life=[SECS] ...
//! ```
//!

use super::{super::*, run};
use crate::{get_servaddr, resp_print};
use myutil::{err::*, *};
use std::{collections::HashSet, mem, time};

///////////////////////////////
#[derive(Default)]
pub struct EnvUpdate<'a> {
    pub env_set: Vec<&'a EnvIdRef>,
    pub cpu_num: Option<u8>,
    pub mem_size: Option<u16>,
    pub vm_port: Vec<u16>,
    /// for kick-vm
    pub kick_dead: bool,
    /// for kick-vm
    pub vm_id: Vec<VmId>,
    /// for kick-vm
    pub os_prefix: Vec<String>,
    /// for life-time
    pub life_time: Option<u64>,
    /// for life-time
    pub is_fucker: bool,
    /// deny VM to do outgoing network-ops
    pub deny_outgoing: Option<bool>,
}
///////////////////////////////

impl<'a> EnvUpdate<'a> {
    /// 发送请求并打印结果
    pub fn do_req(mut self) -> Result<()> {
        mem::take(&mut self.env_set).iter().for_each(|env| {
            if let Some(life_time) = self.life_time {
                let res = get_ops_id("update_env_lifetime")
                    .c(d!())
                    .and_then(|ops_id| {
                        get_servaddr().c(d!()).and_then(|addr| {
                            send_req(
                                ops_id,
                                gen_req(ReqUpdateEnvLife {
                                    env_id: env.to_string(),
                                    life_time,
                                    is_fucker: self.is_fucker,
                                }),
                                addr,
                            )
                            .c(d!())
                        })
                    })
                    .and_then(|resp| resp_print!(resp, String));
                info_omit!(res);
            }

            if !self.vm_port.is_empty()
                || self.cpu_num.is_some()
                || self.mem_size.is_some()
                || self.deny_outgoing.is_some()
            {
                let res = get_ops_id("update_env_resource")
                    .c(d!())
                    .and_then(|ops_id| {
                        get_servaddr().c(d!()).and_then(|addr| {
                            send_req(
                                ops_id,
                                gen_req(ReqUpdateEnvResource {
                                    env_id: env.to_string(),
                                    cpu_num: self.cpu_num.map(|n| n as i32),
                                    mem_size: self.mem_size.map(|i| i as i32),
                                    disk_size: None,
                                    vm_port: self.vm_port.clone(),
                                    deny_outgoing: self.deny_outgoing,
                                }),
                                addr,
                            )
                            .c(d!())
                        })
                    })
                    .and_then(|resp| resp_print!(resp, String));
                info_omit!(res);
            }

            if !self.os_prefix.is_empty() || !self.vm_id.is_empty() {
                let res = get_ops_id("update_env_kick_vm")
                    .c(d!())
                    .and_then(|ops_id| {
                        get_servaddr().c(d!()).and_then(|addr| {
                            send_req(
                                ops_id,
                                gen_req(ReqUpdateEnvKickVm {
                                    env_id: env.to_string(),
                                    vm_id: self.vm_id.clone(),
                                    os_prefix: self.os_prefix.clone(),
                                }),
                                addr,
                            )
                            .c(d!())
                        })
                    })
                    .and_then(|resp| resp_print!(resp, String));
                info_omit!(res);
            }

            // - `tt env run <ENV> -c 'date' -t 5`,
            // - 收集所有超时未返回的实例集合,
            // - `tt env update <ENV> --kick-vm=...`
            if self.kick_dead {
                let res =
                    run::get_conn_info(&[env]).c(d!()).and_then(|vci_set| {
                        let vmid_set = vci_set
                            .iter()
                            .map(|vci| vci.id)
                            .collect::<HashSet<_>>();

                        let hdr = run::ttrexec::exec("date", vci_set);
                        let running_set = if hdr
                            .recv_timeout(time::Duration::from_secs(8))
                            .is_err()
                        {
                            HashSet::new()
                        } else {
                            (1..vmid_set.len())
                                .filter_map(|_| {
                                    hdr.recv_timeout(
                                        time::Duration::from_secs(5),
                                    )
                                    .ok()
                                })
                                .filter(|vci| 0 == vci.status_code)
                                .map(|vci| vci.id)
                                .collect::<HashSet<_>>()
                        };

                        let to_kick = vmid_set
                            .difference(&running_set)
                            .copied()
                            .collect::<Vec<_>>();

                        EnvUpdate {
                            env_set: vct![&env],
                            cpu_num: None,
                            mem_size: None,
                            vm_port: vct![],
                            kick_dead: false,
                            vm_id: to_kick,
                            os_prefix: vct![],
                            life_time: None,
                            is_fucker: false,
                            deny_outgoing: None,
                        }
                        .do_req()
                        .c(d!())
                    });
                info_omit!(res);
            }
        });

        Ok(())
    }
}
