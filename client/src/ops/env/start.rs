//!
//! # Env
//!
//! ```shell
//! tt env del ...
//! ```
//!

use super::super::*;
use crate::{get_servaddr, resp_print};
use myutil::{err::*, *};

///////////////////////////////
#[derive(Default)]
pub struct EnvStart<'a> {
    pub env_set: Vec<&'a EnvIdRef>,
}
///////////////////////////////

impl<'a> EnvStart<'a> {
    /// 发送请求并打印结果
    pub fn do_req(&self) -> Result<()> {
        self.env_set.iter().for_each(|env| {
            info_omit!(
                get_ops_id("start_env")
                    .c(d!())
                    .and_then(|ops_id| {
                        get_servaddr().c(d!()).and_then(|addr| {
                            send_req(
                                ops_id,
                                gen_req(ReqStartEnv {
                                    env_id: env.to_string(),
                                }),
                                addr,
                            )
                            .c(d!())
                        })
                    })
                    .and_then(|resp| resp_print!(resp, String))
            )
        });
        Ok(())
    }
}
