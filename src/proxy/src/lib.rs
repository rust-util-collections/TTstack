//!
//! # tt-proxy
//!
//! 为多个 tt-server 做前端代理, 统一调度全局资源.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

pub mod cfg;
mod def;
mod hdr;
mod http;
mod util;

use async_std::{
    future,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
    task,
};
use def::Proxy;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::Mutex;
use std::{
    os::unix::io::{IntoRawFd, RawFd},
    sync::atomic::{AtomicU32, Ordering},
    sync::mpsc::channel,
    thread,
    time::Duration,
};
use ttserver_def::*;
use ttutils::zlib;

// recv timeout in seconds
const RECV_TO_SECS: u64 = 12;

lazy_static! {
    static ref CFG: &'static cfg::Cfg = pnk!(cfg::register_cfg(None));
    /// 与客户端交互
    static ref SOCK: UdpSocket = pnk!(gen_master_sock());
    /// HTTP 与 UDP 发来的客户端消息, 处理流程一致,
    /// 都经由此 Unix(Unix Domain) Abstract Socket 中转
    static ref SOCK_UAU: RawFd = pnk!(util::gen_uau_socket(include_bytes!("uau.addr"))).0.into_raw_fd();
    /// 与后端的 TT Slave 服务端交互
    static ref SOCK_MID: UdpSocket = pnk!(gen_middle_sock());
    static ref PROXY: Arc<Mutex<Proxy>> =
        Arc::new(Mutex::new(Proxy::default()));
    /// Number bytes will be used as an uau address
    static ref UAU_ID: AtomicU32 = AtomicU32::new(0);
}

/// Entry Point
pub fn start(cfg: cfg::Cfg) -> Result<()> {
    pnk!(cfg::register_cfg(Some(cfg)));

    thread::spawn(|| {
        hdr::sync::start_cron();
    });

    start_middle_serv();
    start_serv_udp();

    // will block
    start_serv_http();

    Ok(())
}

/// 与 Slave Server 通信
fn start_middle_serv() {
    // 处理 Slave Server 回复的信息
    fn deal_slave_resp(
        peeraddr: SocketAddr,
        slave_resp: Vec<u8>,
    ) -> Result<()> {
        zlib::decode(&slave_resp[..])
            .c(d!())
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
        // At most 64KB can be sent on UDP/INET[4/6]
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

/// 与 Cli 端交互,
/// 使用 Http/TCP 协议
#[inline(always)]
fn start_serv_http() {
    let mut app = tide::new();
    app.at("/register_client_id").post(http::register_client_id);
    app.at("/get_server_info").post(http::get_server_info);
    app.at("/get_env_list").post(http::get_env_list);
    app.at("/get_env_info").post(http::get_env_info);
    app.at("/add_env").post(http::add_env);
    app.at("/del_env").post(http::del_env);
    app.at("/update_env_lifetime")
        .post(http::update_env_lifetime);
    app.at("/update_env_kick_vm").post(http::update_env_kick_vm);
    app.at("/get_env_list_all").post(http::get_env_list_all);
    app.at("/stop_env").post(http::stop_env);
    app.at("/start_env").post(http::start_env);
    app.at("/update_env_resource")
        .post(http::update_env_resource);

    // As daemon
    pnk!(task::block_on(app.listen(&*CFG.proxy_serv_at)));
}

/// 与 Cli 端交互,
/// 使用 Udp 协议
#[inline(always)]
fn start_serv_udp() {
    task::spawn(async {
        let mut buf = vec![0; 8192];
        loop {
            if let Ok((size, peeraddr)) = info!(SOCK.recv_from(&mut buf).await)
            {
                if size < OPS_ID_LEN {
                    p(eg!(format!("Invalid request from {}", peeraddr)));
                    continue;
                }

                parse_ops_id(&buf[0..OPS_ID_LEN])
                    .c(d!())
                    .and_then(|ops_id| {
                        let recvd =
                            zlib::decode(&buf[OPS_ID_LEN..size]).c(d!())?;
                        task::spawn(async move {
                            info_omit!(
                                serv_it_udp(ops_id, recvd, peeraddr).await
                            );
                        });
                        Ok(())
                    })
                    .unwrap_or_else(|e| p(e));
            }
        }
    });
}

// 保持与 HTTP 相同的处理流程,
// 以简化底层核心数据结构设计
#[inline(always)]
async fn serv_it_udp(
    ops_id: usize,
    request: Vec<u8>,
    peeraddr: SocketAddr,
) -> Result<()> {
    let (mysock, myaddr) = match util::gen_uau_socket(
        &UAU_ID.fetch_add(1, Ordering::Relaxed).to_ne_bytes(),
    )
    .c(d!())
    {
        Ok((s, a)) => (s, a),
        Err(e) => {
            return send_err!(@DEFAULT_REQ_ID, e, peeraddr);
        }
    };

    if let Some(ops) = hdr::OPS_MAP.get(ops_id) {
        ops(ops_id, myaddr, request)
            .c(d!())
            .or_else(|e| send_err!(@DEFAULT_REQ_ID, e, peeraddr))?;

        let mut buf = vec![0; 64 * 1024];
        match future::timeout(
            Duration::from_secs(RECV_TO_SECS),
            mysock.recv(&mut buf),
        )
        .await
        .c(d!())?
        .c(d!())
        {
            Ok(siz) => SOCK
                .send_to(&buf[..siz], peeraddr)
                .await
                .c(d!())
                .map(|_| ()),
            Err(e) => send_err!(@DEFAULT_REQ_ID, e, peeraddr),
        }
    } else {
        send_err!(@DEFAULT_REQ_ID, eg!("Invalid operation-ID !!!"), peeraddr)
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
