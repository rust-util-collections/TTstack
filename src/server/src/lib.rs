//!
//! # tt-server
//!
//! 处理与 Client 端的交互逻辑.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

pub mod cfg;
mod def;
mod hdr;
mod init;
pub mod util;

use def::{DEFAULT_REQ_ID, OPS_ID_LEN};
use futures::executor::ThreadPool;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use std::{
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};
use ttutils::zlib;

lazy_static! {
    static ref POOL: ThreadPool = pnk!(util::gen_thread_pool(Some(8)));
    static ref CFG: &'static cfg::Cfg = pnk!(cfg::register_cfg(None));
    static ref SERV: Arc<ttcore::Serv> = Arc::new(ttcore::Serv::new());
    static ref SOCK: UdpSocket = pnk!(UdpSocket::bind(&CFG.serv_at).c(d!()));
}

/// 服务启动入口
pub fn start(cfg: cfg::Cfg) -> Result<()> {
    pnk!(cfg::register_cfg(Some(cfg)));

    init::setenv()
        .c(d!())
        .and_then(|_| ttcore::exec(&CFG.image_path, run, &CFG.serv_ip))
}

#[inline(always)]
fn run() -> Result<()> {
    // 必须在 clone 调用之后执行,
    // 否则会导致 POOL 在父进程中被初始化,
    // 进入子进程后只会保留一个主线程,
    // 且 lazy_static 不会再次初始化线程池.
    init::start_cron();

    // (C/S) 网络交互
    start_netserv();
}

fn start_netserv() -> ! {
    let mut buf = vec![0; 8192];
    loop {
        if let Ok((size, peeraddr)) = SOCK.recv_from(&mut buf) {
            if size < OPS_ID_LEN {
                continue;
            }
            parse_ops_id(&buf[..OPS_ID_LEN])
                .c(d!())
                .and_then(|ops_id| {
                    let recvd =
                        zlib::decode(&buf[OPS_ID_LEN..size]).c(d!())?;
                    POOL.spawn_ok(async move {
                        info_omit!(serv_it(ops_id, recvd, peeraddr).await);
                    });
                    Ok(())
                })
                .unwrap_or_else(|e| p(e));
        }
    }
}

/// 处理 Client 请求
#[inline(always)]
async fn serv_it(
    ops_id: usize,
    msg_body: Vec<u8>,
    peeraddr: SocketAddr,
) -> Result<()> {
    hdr::OPS_MAP
        .get(ops_id)
        .ok_or(eg!(format!("Unknown Request: {}", ops_id)))
        .and_then(|ops| ops(peeraddr, msg_body).c(d!()))
        .or_else(|e| send_err!(DEFAULT_REQ_ID, e, peeraddr))
}

#[inline(always)]
fn parse_ops_id(raw: &[u8]) -> Result<usize> {
    String::from_utf8_lossy(raw).parse::<usize>().c(d!())
}
