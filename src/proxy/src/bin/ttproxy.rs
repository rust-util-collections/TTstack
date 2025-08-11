//!
//! # tt-proxy
//!
//! 为多个 tt-server 做前端代理, 统一调度全局资源.
//!

use clap::{Arg, Command};
use ruc::*;
use std::net::SocketAddr;
use ttproxy::cfg::Cfg;

fn main() {
    pnk!(ttproxy::start(pnk!(parse_cfg())))
}

/// 解析命令行参数
pub(crate) fn parse_cfg() -> Result<Cfg> {
    // 要添加 "--ignored" 等兼容 `cargo test` 的选项
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::new("proxy-addr")
            .long("proxy-addr")
            .value_name("ADDR")
            .help("ttproxy 地址, eg: 127.0.0.1:19527."))
        .arg(Arg::new("server-set")
            .long("server-set")
            .value_name("ADDR")
            .action(clap::ArgAction::Append)
            .help("ttserver 地址, eg: 127.0.0.1:9527,10.10.10.101:9527."))
        .get_matches();

    match (
        matches.get_one::<String>("proxy-addr"),
        matches.get_many::<String>("server-set"),
    ) {
        (Some(proxy_addr), Some(server_set)) => {
            let (proxy_serv_at, server_addr_set, server_set) = {
                let mut set = vct![];
                let mut orig_set = vct![];
                for s in server_set {
                    set.push(s.parse::<SocketAddr>().c(d!())?);
                    orig_set.push(s.to_owned());
                }
                (proxy_addr.clone(), set, orig_set)
            };
            Ok(Cfg {
                proxy_serv_at,
                server_addr_set,
                server_set,
            })
        }
        _ => Err(eg!()),
    }
}
