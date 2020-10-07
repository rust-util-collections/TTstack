//!
//! # Env
//!
//! ```shell
//! tt env show ...
//! ```
//!

use super::super::*;
use crate::{get_servaddr, resp_parse, resp_print};
use myutil::{err::*, *};
use std::collections::BTreeMap;

///////////////////////////////
#[derive(Default)]
pub struct EnvShow<'a> {
    /// 目前会忽略此参数, 总是返回所有 Env 的信息
    pub env_set: Vec<&'a EnvIdRef>,
}
///////////////////////////////

impl<'a> EnvShow<'a> {
    /// 发送请求并打印结果
    #[inline(always)]
    pub fn do_req(self) -> Result<()> {
        get_res(&self.env_set).map(|r| resp_print!(r))
    }
}

/// 拆出此功能函数供 run 和 deploy 复用
pub(super) fn get_res(
    env_set: &[&EnvIdRef],
) -> Result<BTreeMap<ServerAddr, RespGetEnvInfo>> {
    get_ops_id("get_env_info")
        .c(d!())
        .and_then(|ops_id| get_servaddr().c(d!()).map(|addr| (ops_id, addr)))
        .and_then(|(ops_id, addr)| {
            send_req(
                ops_id,
                gen_req(ReqGetEnvInfo {
                    env_set: env_set.iter().map(|id| id.to_string()).collect(),
                }),
                addr,
            )
            .c(d!())
        })
        .and_then(
            |resp| resp_parse!(resp, BTreeMap<ServerAddr, RespGetEnvInfo>),
        )
}
