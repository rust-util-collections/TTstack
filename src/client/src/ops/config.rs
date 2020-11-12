//!
//! # Config Request
//!

use super::*;
use crate::{read_cfg, write_cfg, Server};
use myutil::{err::*, *};
use std::{env, net::SocketAddr, process};

///////////////////////////////
#[derive(Default, Debug)]
pub struct Config<'a> {
    pub server_addr: &'a str,
    pub server_port: u16,
    pub client_id: &'a str,
}
///////////////////////////////

impl<'a> Config<'a> {
    /// - 本地信息直接写入配置文件
    /// - 更新 server 信息时, 需验证其有效性
    pub fn do_req(&mut self) -> Result<()> {
        if "" == self.client_id && "" == read_cfg().c(d!())?.client_id {
            return Err(eg!("Client ID can't be empty !!!"));
        }

        if "" != self.client_id {
            read_cfg().c(d!()).and_then(|mut cfg| {
                // 尽可能避免分布式环境中的重复事件:
                // 用户指定的ID + timestamp + processId + $USER
                cfg.client_id = format!(
                    "{}@{}@{}@{}",
                    self.client_id,
                    ts!(),
                    process::id(),
                    env::var("USER").unwrap_or_else(|_| "".to_owned())
                );
                write_cfg(&cfg).c(d!())
            })?;
        }

        if "" != self.server_addr {
            alt!(0 == self.server_port, self.server_port = 9527);
            get_ops_id("register_client_id")
                .c(d!())
                .and_then(|ops_id| {
                    format!("{}:{}", self.server_addr, self.server_port)
                        .parse::<SocketAddr>()
                        .map(|addr| (ops_id, addr))
                        .c(d!("Invalid server_addr OR server_port"))
                })
                .and_then(|(ops_id, addr)| {
                    send_req::<&str>(ops_id, gen_req(""), addr).c(d!())
                })
                .and_then(|_| read_cfg().c(d!()))
                .and_then(|mut cfg| {
                    cfg.server_list =
                        Server::new(self.server_addr, self.server_port);
                    write_cfg(&cfg).c(d!())
                })?;
        }

        Ok(())
    }
}
