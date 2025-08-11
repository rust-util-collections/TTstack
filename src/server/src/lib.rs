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
mod util;

use def::{DEFAULT_REQ_ID, OPS_ID_LEN};
use futures::executor::ThreadPool;
use ruc::*;
use std::{
    mem,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, LazyLock},
};
use ttutils::zlib;
use util::{genlog, p};

static POOL: LazyLock<ThreadPool> = LazyLock::new(|| pnk!(util::gen_thread_pool(Some(8))));
static CFG: LazyLock<&'static cfg::Cfg> = LazyLock::new(|| pnk!(cfg::register_cfg(None)));
static SERV: LazyLock<Arc<ttcore::Serv>> = LazyLock::new(|| Arc::new(ttcore::Serv::new(&CFG.cfgdb_path)));
static SOCK: LazyLock<UdpSocket> = LazyLock::new(|| pnk!(UdpSocket::bind(&CFG.serv_at).c(d!())));

/// 服务启动入口
pub fn start(cfg: cfg::Cfg) -> ruc::Result<()> {
    pnk!(cfg::register_cfg(Some(cfg)));
    init::setenv()
        .c(d!())
        .and_then(|_| ttcore::exec(&CFG.image_path, run, &CFG.serv_ip))
}

#[inline(always)]
fn run() -> ruc::Result<()> {
    // 必须在 clone 调用之后执行,
    // 否则会导致 POOL 在父进程中被初始化,
    // 进入子进程后只会保留一个主线程,
    // 且 LazyLock 不会再次初始化线程池.
    init::start_cron();

    // 必须在 clone 调用之后执行,
    // 同样是因为 LazyLock 所限,
    load_exists().c(d!())?;

    // (C/S) 网络交互
    start_netserv();
}

// 载入先前已存在的 ENV 实例
fn load_exists() -> ruc::Result<()> {
    let mut vm_set;
    for (cli, env_set) in SERV.cfg_db.read_all().c(d!())?.into_iter() {
        for mut env in env_set.into_iter() {
            vm_set = mem::take(&mut env.vm)
                .into_iter()
                .map(|(_, vm_set)| vm_set)
                .collect();
            env.load(&SERV)
                .c(d!())
                .and_then(|mut env| {
                    env.add_vm_set_complex(vec![], vm_set, true)
                        .c(d!())
                        .map(|_| env)
                })
                .and_then(|env| SERV.register_env(cli.clone(), env).c(d!()))?;
        }
    }

    Ok(())
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
                .unwrap_or_else(|e| { p(e); });
        }
    }
}

/// 处理 Client 请求
#[inline(always)]
async fn serv_it(
    ops_id: usize,
    msg_body: Vec<u8>,
    peeraddr: SocketAddr,
) -> ruc::Result<()> {
    hdr::OPS_MAP
        .get(ops_id)
        .ok_or(eg!(format!("Unknown Request: {}", ops_id)))
        .and_then(|ops| ops(peeraddr, msg_body).c(d!()))
        .or_else(|e| send_err!(DEFAULT_REQ_ID, e, peeraddr))
}

#[inline(always)]
fn parse_ops_id(raw: &[u8]) -> ruc::Result<usize> {
    String::from_utf8_lossy(raw).parse::<usize>().c(d!())
}
