//!
//! # Commiuncation With Client
//!

use crate::{def::*, POOL, SOCK};
use futures::executor::{ThreadPool, ThreadPoolBuilder};
use futures_timer::Delay;
use myutil::{err::*, *};
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
use ttutils::zlib;

/// 生成异步框架底层的线程池
pub(crate) fn gen_thread_pool(n: Option<u8>) -> Result<ThreadPool> {
    ThreadPoolBuilder::new()
        .pool_size(n.map(|n| n as usize).unwrap_or_else(num_cpus::get))
        .create()
        .c(d!())
}

/// 回送成功信息
#[macro_export(crate)]
macro_rules! send_ok {
    ($uuid: expr, $msg: expr, $peeraddr: expr) => {
        $crate::util::send_back(
            $crate::util::gen_resp_ok($uuid, $msg),
            $peeraddr,
        )
    };
}

/// 生成标志'成功'的回复体
pub(crate) fn gen_resp_ok(uuid: u64, msg: impl Serialize) -> Resp {
    Resp {
        uuid,
        status: RetStatus::Success,
        msg: info!(serde_json::to_vec(&msg)).unwrap_or_default(),
    }
}

/// 回送失败信息
#[macro_export(crate)]
macro_rules! send_err {
    ($uuid: expr, $err: expr, $peeraddr: expr) => {{
        let log = genlog($err);
        $crate::util::send_back(
            $crate::util::gen_resp_err($uuid, &log),
            $peeraddr,
        )
        .c(d!(&log))
        .map(|_| p(eg!(log)))
    }};
}

/// 生成标志'出错'的回复体
pub(crate) fn gen_resp_err(uuid: u64, msg: &str) -> Resp {
    Resp {
        uuid,
        status: RetStatus::Fail,
        msg: msg.as_bytes().to_vec(),
    }
}

/// 回送信息至请求方(Client/Proxy)
#[inline(always)]
pub(crate) fn send_back(resp: Resp, peeraddr: SocketAddr) -> Result<()> {
    serde_json::to_vec(&resp)
        .c(d!())
        .and_then(|resp| zlib::encode(&resp[..]).c(d!()))
        .map(|resp_compressed| {
            POOL.spawn_ok(async move {
                info_omit!(SOCK.send_to(&resp_compressed, peeraddr));
            });
        })
}

/// 异步 sleep
pub(crate) async fn asleep(sec: u64) {
    Delay::new(Duration::from_secs(sec)).await;
}
