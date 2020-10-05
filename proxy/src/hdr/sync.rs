//!
//! 定时同步各 Slave Server 的资源信息
//!

use super::*;
use crate::{fwd_to_slave, CFG};
use async_std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{thread, time::Duration};
use ttserver_def::*;

// 每秒同步一次
const SYNC_ITV: u64 = 1;

pub(crate) fn start_cron() {
    // 一个空请求体即可
    let mut req = Req::new(0, "");

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

    // 从 Slave 正常返回的信息优先,
    // 未正常返回的 Slave 信息保留原值
    let mut map = ENV_MAP.write();
    res.into_iter().for_each(|(k, v)| {
        map.insert(k, v);
    });
}

#[inline(always)]
fn mock_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 35107)
}
