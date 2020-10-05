//!
//! # Status Request
//!

use super::*;
use crate::{get_servaddr, resp_parse, resp_print, CFG};
use myutil::{err::*, *};
use std::collections::HashMap;

///////////////////////////////
#[derive(Default)]
pub struct Status {
    #[allow(dead_code)]
    pub client: bool,
    pub server: bool,
}
///////////////////////////////

impl Status {
    /// 发送请求并打印结果
    pub fn do_req(&self) -> Result<()> {
        if self.server {
            self.get_res().c(d!()).map(|mut r| {
                r.values_mut().for_each(|si| {
                    si.supported_list.sort();
                });
                resp_print!(r)
            })
        } else {
            CFG.print_to_user();
            Ok(())
        }
    }

    fn get_res(&self) -> Result<HashMap<ServerAddr, RespGetServerInfo>> {
        let addr = get_servaddr().c(d!())?;
        get_ops_id("get_server_info")
            .c(d!())
            .and_then(|ops_id| {
                send_req::<&str>(ops_id, gen_req(""), addr).c(d!())
            })
            .and_then(|resp| resp_parse!(resp, HashMap<ServerAddr, RespGetServerInfo>))
    }
}
