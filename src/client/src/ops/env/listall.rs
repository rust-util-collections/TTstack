//!
//! # Env
//!
//! ```shell
//! tt env listall
//! ```
//!

use super::super::*;
use crate::{get_servaddr, resp_parse, resp_print};
use myutil::{err::*, *};
use std::collections::HashMap;

///////////////////////////////
pub struct EnvListAll;
///////////////////////////////

impl EnvListAll {
    /// 发送请求并打印结果
    pub fn do_req(&self) -> Result<()> {
        self.get_res().c(d!()).map(|r| resp_print!(r))
    }

    /// 发送请求并获取结果
    pub fn get_res(&self) -> Result<HashMap<ServerAddr, RespGetEnvList>> {
        get_ops_id("get_env_list_all")
            .c(d!())
            .and_then(|ops_id| {
                get_servaddr().c(d!()).and_then(|addr| {
                    send_req::<&str>(ops_id, gen_req(""), addr).c(d!())
                })
            })
            .and_then(
                |resp| resp_parse!(resp, HashMap<ServerAddr, RespGetEnvList>),
            )
    }
}
