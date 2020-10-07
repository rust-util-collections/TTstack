//!
//! # Commiuncation With Client
//!

use crate::SOCK;
use async_std::{net::SocketAddr, task};
use flate2::{write::ZlibEncoder, Compression};
use myutil::{err::*, *};
use serde::Serialize;
use std::{io::Write, time::Duration};
use ttserver_def::*;

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

/// 回送信息
#[inline(always)]
pub(crate) fn send_back(resp: Resp, peeraddr: SocketAddr) -> Result<()> {
    serde_json::to_vec(&resp)
        .c(d!())
        .and_then(|resp| {
            let mut en = ZlibEncoder::new(vct![], Compression::default());
            en.write_all(&resp[..]).c(d!())?;
            en.finish().c(d!())
        })
        .map(|resp_compressed| {
            task::spawn(async move {
                info_omit!(SOCK.send_to(&resp_compressed, peeraddr).await);
            });
        })
}

#[inline(always)]
pub(crate) async fn asleep(secs: u64) {
    task::sleep(Duration::from_secs(secs)).await
}
