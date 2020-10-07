//!
//! # tt-proxy
//!
//! 为多个 tt-server 做前端代理, 统一调度全局资源.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

pub mod cfg;
mod def;
mod hdr;
mod util;

use async_std::{
    net::{SocketAddr, UdpSocket},
    sync::Arc,
    task,
};
use def::Proxy;
use flate2::write::ZlibDecoder;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::Mutex;
use std::{io::Write, sync::mpsc::channel};
use ttserver_def::*;

lazy_static! {
    static ref CFG: &'static cfg::Cfg = pnk!(cfg::register_cfg(None));
    static ref SOCK: UdpSocket = pnk!(gen_master_sock());
    static ref SOCK_MID: UdpSocket = pnk!(gen_middle_sock());
    static ref PROXY: Arc<Mutex<Proxy>> =
        Arc::new(Mutex::new(Proxy::default()));
}

/// Entry Point
pub fn start(cfg: cfg::Cfg) -> Result<()> {
    pnk!(cfg::register_cfg(Some(cfg)));

    start_middle_serv();
    task::spawn(start_serv());

    // this is a loop
    hdr::sync::start_cron();
    Ok(())
}

/// 与 Slave Server 通信
fn start_middle_serv() {
    // 处理 Slave Server 回复的信息
    fn deal_slave_resp(
        peeraddr: SocketAddr,
        slave_resp: Vec<u8>,
    ) -> Result<()> {
        let mut d = ZlibDecoder::new(vct![]);
        d.write_all(&slave_resp[..])
            .c(d!())
            .and_then(|_| d.finish().c(d!()))
            .and_then(|resp_decompressed| {
                serde_json::from_slice::<Resp>(&resp_decompressed).c(d!())
            })
            .and_then(|resp| {
                let uuid = resp.uuid;
                let mut proxy = PROXY.lock();

                // 在加锁情况下, 只要 UUID 还在, 就表明还未过期,
                // 此处不能理更新 Bucket 的时间戳.
                let idx = *proxy.idx_map.get(&resp.uuid).ok_or(eg!())?;

                let slave_res =
                    proxy.buckets[idx].res.get_mut(&resp.uuid).ok_or(eg!())?;
                slave_res.num_to_wait -= 1;
                slave_res.msg.insert(peeraddr, resp);

                // 已收集齐所有 Slave 的回复,
                // 丢弃实体以触发 Drop 回复 Client
                if 0 == slave_res.num_to_wait {
                    proxy.buckets[idx].res.remove(&uuid);
                }

                Ok(())
            })
    }

    task::spawn(async {
        // At most 64KB can be sent on UDP
        let mut buf = vec![0; 64 * 1024];
        loop {
            if let Ok((size, peeraddr)) =
                info!(SOCK_MID.recv_from(&mut buf).await)
            {
                let recvd = buf[..size].to_vec();
                task::spawn(async move {
                    info_omit!(deal_slave_resp(peeraddr, recvd));
                });
            }
        }
    });

    // 每秒定时清理过期信息
    task::spawn(async {
        loop {
            util::asleep(1).await;
            PROXY.lock().clean_timeout();
        }
    });
}

/// 主线程 Daemon
#[inline(always)]
async fn start_serv() -> Result<()> {
    let mut buf = vec![0; 8192];

    loop {
        if let Ok((size, peeraddr)) = info!(SOCK.recv_from(&mut buf).await) {
            if size < OPS_ID_LEN {
                p(eg!(format!("Invalid request from {}", peeraddr)));
                continue;
            }

            parse_ops_id(&buf[0..OPS_ID_LEN])
                .c(d!())
                .map(|ops_id| {
                    let recvd = buf[OPS_ID_LEN..size].to_vec();
                    task::spawn(async move {
                        info_omit!(serv_it(ops_id, recvd, peeraddr));
                    });
                })
                .unwrap_or_else(|e| p(e));
        }
    }
}

#[inline(always)]
fn serv_it(
    ops_id: usize,
    request: Vec<u8>,
    peeraddr: SocketAddr,
) -> Result<()> {
    if let Some(ops) = hdr::OPS_MAP.get(ops_id) {
        ops(ops_id, peeraddr, request)
            .c(d!())
            .or_else(|e| send_err!(DEFAULT_REQ_ID, e, peeraddr))
    } else {
        send_err!(DEFAULT_REQ_ID, eg!("Invalid operation-id !"), peeraddr)
    }
}

#[inline(always)]
fn parse_ops_id(raw: &[u8]) -> Result<usize> {
    String::from_utf8_lossy(raw).parse::<usize>().c(d!())
}

/// 生成与 Client 通信的套接字
fn gen_master_sock() -> Result<UdpSocket> {
    let (s, r) = channel();
    task::spawn(async move {
        let sock = UdpSocket::bind(&CFG.proxy_serv_at).await;
        info_omit!(s.send(sock));
    });
    if let Ok(Ok(sock)) = r.recv() {
        return Ok(sock);
    }
    Err(eg!())
}

/// 生成与 Slave Server 通信的套接字
fn gen_middle_sock() -> Result<UdpSocket> {
    let (s, r) = channel();
    let mut addr;
    for port in (20_000 + ts!() % 10_000)..60_000 {
        addr = SocketAddr::from(([0, 0, 0, 0], port as u16));
        let ss = s.clone();
        task::spawn(async move {
            let sock = UdpSocket::bind(addr).await;
            info_omit!(ss.send(sock));
        });

        if let Ok(Ok(sock)) = r.recv() {
            return Ok(sock);
        }
    }
    Err(eg!())
}
