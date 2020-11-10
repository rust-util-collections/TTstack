//!
//! # Env
//!
//! ```shell
//! tt env add ...
//! ```
//!

use super::super::*;
use crate::{get_servaddr, resp_print};
use myutil::{err::*, *};

///////////////////////////////
#[derive(Default)]
pub struct EnvAdd<'a> {
    pub env_id: &'a str,
    pub os_prefix: Vec<&'a str>,
    pub vm_port: Vec<u16>,
    /// 0 代表使用服务端预设的默认值
    pub life_time: u64,
    /// 0 代表使用服务端预设的默认值
    pub cpu_num: u8,
    /// 0 代表使用服务端预设的默认值
    pub mem_size: u16,
    /// 0 代表使用服务端预设的默认值
    pub disk_size: u32,
    /// 0 代表使用服务端预设的默认值
    pub dup_each: u16,
    /// 是否禁止 VM 对外连网
    pub deny_outgoing: bool,
    /// VM uuid 是否随机化(唯一)
    pub rand_uuid: bool,
}
///////////////////////////////

impl<'a> EnvAdd<'a> {
    /// 发送请求并打印结果
    pub fn do_req(self) -> Result<()> {
        if 0 < self.life_time && 30 > self.life_time {
            return Err(eg!("Life time too short(min: 30) !"));
        }

        if 12 < self.cpu_num {
            return Err(eg!("Cpu number too large(max: 12) !"));
        }

        if 0 < self.mem_size && 50 > self.mem_size {
            return Err(eg!("Memory size too small(min: 50MB) !"));
        }

        if 48 * 1024 < self.mem_size {
            return Err(eg!("Memory size too large(max: 48GB) !"));
        }

        if 0 < self.disk_size && 100 > self.disk_size {
            return Err(eg!("Disk size too small(min: 100) !"));
        }

        if 1024 * 1024 < self.disk_size {
            return Err(eg!("Disk size too large(max: 1TB) !"));
        }

        get_ops_id("add_env")
            .c(d!())
            .and_then(|ops_id| {
                get_servaddr().c(d!()).and_then(|addr| {
                    send_req(ops_id, gen_req(ReqAddEnv::from(self)), addr)
                        .c(d!())
                })
            })
            .and_then(|resp| resp_print!(resp, String))
    }
}

impl<'a> From<EnvAdd<'a>> for ReqAddEnv {
    fn from(v: EnvAdd<'a>) -> Self {
        ReqAddEnv {
            env_id: v.env_id.to_owned(),
            os_prefix: v.os_prefix.into_iter().map(|s| s.to_owned()).collect(),
            life_time: alt!(0 == v.life_time, None, Some(v.life_time)),
            cpu_num: alt!(0 == v.cpu_num, None, Some(v.cpu_num as i32)),
            mem_size: alt!(0 == v.mem_size, None, Some(v.mem_size as i32)),
            disk_size: alt!(0 == v.disk_size, None, Some(v.disk_size as i32)),
            port_set: v.vm_port,
            dup_each: alt!(0 == v.dup_each, None, Some(v.dup_each as u32)),
            deny_outgoing: v.deny_outgoing,
            rand_uuid: v.rand_uuid,
            vmcfg: None,
        }
    }
}
