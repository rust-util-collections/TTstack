//!
//! # tt-proxy
//!
//! 为多个 tt-server 做前端代理, 统一调度全局资源.
//!

use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use myutil::{err::*, *};
use std::net::SocketAddr;
use ttproxy::cfg::Cfg;

fn main() {
    pnk!(ttproxy::start(pnk!(parse_cfg())))
}

/// 解析命令行参数
pub(crate) fn parse_cfg() -> Result<Cfg> {
    // 要添加 "--ignored" 等兼容 `cargo test` 的选项
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .args_from_usage("--proxy-addr=[ADDR] 'ttproxy 地址, eg: 127.0.0.1:19527.'")
        .args_from_usage("--server-set=[ADDR]... 'ttserver 地址, eg: 127.0.0.1:9527,10.10.10.101:9527.'")
        .get_matches();

    match (
        matches.value_of("proxy-addr"),
        matches.values_of("server-set"),
    ) {
        (Some(proxy_addr), Some(server_set)) => {
            let (proxy_serv_at, server_addr_set, server_set) = {
                let mut set = vct![];
                let mut orig_set = vct![];
                for s in server_set {
                    set.push(s.parse::<SocketAddr>().c(d!())?);
                    orig_set.push(s.to_owned());
                }
                (proxy_addr.to_owned(), set, orig_set)
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
