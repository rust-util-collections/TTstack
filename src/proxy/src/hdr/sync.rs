//!
//! 定时同步各 Slave Server 的资源信息
//!

use super::*;
use crate::{fwd_to_slave, CFG};
use async_std::net::{IpAddr, Ipv4Addr, SocketAddr};
use nix::sys::socket::{InetAddr, SockAddr};
use std::{thread, time::Duration};
use ttserver_def::*;

// 每秒同步一次
const SYNC_ITV: u64 = 1;

pub(crate) fn start_cron() {
    // 一个空请求体即可
    let mut req = Req::new(0, format!("SYSTEM-CRON-{}", ts!()), "");

    // mock 一个地址
    let peeraddr = mock_addr();

    // 向后台所有的 Slave Server 请求信息
    let addr_set = CFG.server_addr_set.clone();

    loop {
        // get_server_info
        info_omit!(
            fwd_to_slave!(@@@1, req, peeraddr, server_info_cb, &addr_set)
        );

        // get_env_list_all
        info_omit!(fwd_to_slave!(@@@8, req, peeraddr, env_list_cb, &addr_set));

        thread::sleep(Duration::from_secs(SYNC_ITV));
    }
}

fn server_info_cb(r: &mut SlaveRes) {
    let res = r
        .msg
        .iter()
        .filter(|(_, raw)| raw.status == RetStatus::Success)
        .filter_map(|(slave, raw)| {
            info!(serde_json::from_slice::<
                HashMap<ServerAddr, RespGetServerInfo>,
            >(&raw.msg))
            .ok()
            .and_then(|resp| resp.into_iter().next())
            .map(|resp| (*slave, resp.1))
        })
        .collect::<HashMap<_, _>>();

    *SLAVE_INFO.write() = res;
}

fn env_list_cb(r: &mut SlaveRes) {
    let res = r
        .msg
        .iter()
        .filter(|(_, raw)| raw.status == RetStatus::Success)
        .filter_map(|(slave, raw)| {
            info!(
                serde_json::from_slice::<HashMap<ServerAddr, RespGetEnvList>>(
                    &raw.msg
                )
            )
            .ok()
            .and_then(|resp| resp.into_iter().next())
            .map(move |resp| resp.1.into_iter().map(move |ei| (ei.id, *slave)))
        })
        .flatten()
        .fold(map! {}, |mut base: HashMap<EnvId, Vec<SocketAddr>>, new| {
            if let Some(slave) = base.get_mut(&new.0) {
                slave.push(new.1);
            } else {
                base.insert(new.0, vct![new.1]);
            }
            base
        });

    // 无法连通的 Slave 在元信息保留,
    // 是一种干扰, 只保留有效信息即可.
    // 另, 若不全量替换, Slave 端到期自动被清理掉的 ENV,
    // 会依然保留在 Proxy 中, 导致在创建同名 ENV 时出错.
    *ENV_MAP.write() = res;
}

#[inline(always)]
fn mock_addr() -> SockAddr {
    SockAddr::new_inet(InetAddr::from_std(&SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        35107,
    )))
}
