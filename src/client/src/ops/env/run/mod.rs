//!
//! # Run cmd in ENV
//!
//! ```shell
//! tt env run ...
//! ```
//!

pub mod ssh;
pub mod ttrexec;

use super::{super::EnvIdRef, show};
use crate::ops::{SSH_PORT, TTREXEC_PORT};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use std::{
    fs, mem, sync::mpsc::Receiver, sync::Mutex, thread, time::Duration,
};

///////////////////////////////
#[derive(Default)]
pub struct EnvRun<'a> {
    pub cmd: &'a str,
    /// 若文件不为空, 则忽略 cmd 项
    pub script: &'a str,
    pub env_set: Vec<&'a EnvIdRef>,
    pub time_out: u64,
    pub use_ssh: bool,
    /// 将身份标识发送至对应的 VM,
    /// 使 HAWK 监控系统可正确识别其身份
    pub config_hawk: bool,
    /// 交互式串行操作
    pub interactive: bool,
    pub filter_os_prefix: Vec<&'a str>,
    pub filter_vm_id: Vec<i32>,
}
///////////////////////////////

impl<'a> EnvRun<'a> {
    /// 发送请求并打印结果
    pub fn do_req(&self) -> Result<()> {
        let res = if self.config_hawk {
            self.get_res_hawk()
        } else {
            self.get_res()
        };

        let to = self.time_out;
        res.c(d!()).map(|r| {
            r.into_iter()
                .map(|rt| {
                    thread::spawn(move || {
                        let ops = || -> Result<()> {
                            for _ in 0..rt.0 {
                                rt.1.recv_timeout(Duration::from_secs(to))
                                    .map(print_to_user)
                                    .c(d!())?;
                            }
                            Ok(())
                        };
                        info_omit!(ops());
                    })
                })
                .for_each(|tid| tid.join().unwrap());
        })
    }

    /// 发送请求并获取结果
    fn get_res(&self) -> Result<Vec<(usize, Receiver<VmConnInfo>)>> {
        let cmd = if "" != self.script {
            fs::read(self.script)
                .c(d!())
                .and_then(|c| String::from_utf8(c).c(d!()))?
        } else {
            self.cmd.to_owned()
        };

        if "" == cmd && !self.interactive {
            return Err(eg!("Empty cmd!"));
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
                if self.interactive {
                    ssh::exec_interactive(vci_set);
                } else {
                    vct![(vci_set.len(), ssh::exec(&cmd, vci_set))]
                }
            } else {
                vct![(vci_set.len(), ttrexec::exec(&cmd, vci_set))]
            }
        })
    }

    /// 发送请求并获取结果, --config-hawk
    fn get_res_hawk(&self) -> Result<Vec<(usize, Receiver<VmConnInfo>)>> {
        get_conn_info(&self.env_set).c(d!()).map(|mut vci_set| {
            if !self.filter_vm_id.is_empty()
                || !self.filter_os_prefix.is_empty()
            {
                vci_set = vci_set
                    .into_iter()
                    .filter(|vci| {
                        self.filter_vm_id.iter().any(|id| vci.id == *id)
                            || self
                                .filter_os_prefix
                                .iter()
                                .any(|prefix| vci.os.starts_with(prefix))
                    })
                    .collect();
            }

            let fn_ptr = if self.use_ssh {
                ssh::exec
            } else {
                ttrexec::exec
            };

            // 格式: <ENV>_<ENV-StartTime>_<HOST_IP>_<OS>_<VM_ID>
            vci_set.into_iter().fold(vct![], |mut base, new| {
                let cmd = format!(
                    r"
                    printf 'tt_{}_{}_{}_{}_{}' | sed 's/:/-/g' | sed 's/ /-/g' | sed 's/\./_/g' > /.hawk_id;
                    (nohup pkill -9 'hawk-agent'; bash /usr/local/bin/hawk-agent/tool/start.sh; sleep 3; curl http://localhost:46000/plugin/update) &
                    ",
                    &new.env_id, datetime_local!(new.start_ts), &new.addr, &new.os, new.id
                );
                base.push((1, fn_ptr(&cmd, vct![new])));
                base
            })
        })
    }
}

#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VmConnInfo {
    pub id: i32,
    pub env_id: String,
    pub os: String,
    pub addr: String,
    pub ssh_port: u16,
    pub ttrexec_port: u16,
    pub stdout: String,
    pub stderr: String,
    pub status_code: i32,
    pub start_ts: u64,
}

impl VmConnInfo {
    fn new(
        id: i32,
        env_id: String,
        os: String,
        addr: String,
        ssh_port: u16,
        ttrexec_port: u16,
        start_ts: u64,
    ) -> VmConnInfo {
        VmConnInfo {
            id,
            env_id,
            os,
            addr,
            ssh_port,
            ttrexec_port,
            start_ts,
            stdout: "".to_owned(),
            stderr: "".to_owned(),
            status_code: 0,
        }
    }
}

pub fn print_to_user(r: VmConnInfo) {
    lazy_static! {
        static ref LK: Mutex<()> = Mutex::new(());
    }

    if LK.lock().is_ok() {
        let width = 3;
        eprintln!(
            "\x1b[35;01m[ {}:{} ] [ ExitCode: {:>0w$} ] {}\x1b[00m\n\x1b[01m## StdOut ##\x1b[00m\n{}\n\x1b[01m## StdErr ##\x1b[00m\n{}",
            r.addr,
            r.ssh_port,
            r.status_code,
            r.os,
            r.stdout,
            r.stderr,
            w = width,
        );
    } else {
        pnk!(Err(eg!()));
    }
}

/// 通过 ENV 查询填充 addr 和 xx_port 字段
pub fn get_conn_info(id_set: &[&EnvIdRef]) -> Result<Vec<VmConnInfo>> {
    let envinfo_set = show::get_res(id_set).c(d!()).map(|env_set| {
        env_set
            .into_iter()
            .map(|(_, envs)| envs.into_iter())
            .flatten()
    })?;

    let mut res = vct![];
    for mut env in envinfo_set {
        if env.is_stopped {
            return Err(eg!(format!(
                "ENV '{}' is stopped, you should start it first!",
                env.id
            )));
        }

        mem::take(&mut env.vm)
            .into_iter()
            .filter_map(|(id, vm)| {
                vm.port_map.get(&SSH_PORT).copied().and_then(|ssh_port| {
                    vm.port_map.get(&TTREXEC_PORT).copied().map(
                        |ttrexec_port| {
                            VmConnInfo::new(
                                id,
                                env.id.clone(),
                                vm.os,
                                vm.ip.to_string(),
                                ssh_port,
                                ttrexec_port,
                                env.start_timestamp,
                            )
                        },
                    )
                })
            })
            .for_each(|vi| res.push(vi));
    }

    Ok(res)
}
