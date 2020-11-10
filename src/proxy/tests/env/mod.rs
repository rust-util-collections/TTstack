use lazy_static::lazy_static;
use myutil::{err::*, *};
use nix::unistd::{fork, getuid, ForkResult};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    net::{SocketAddr, UdpSocket},
    process, thread,
    time::Duration,
};
use ttproxy::cfg::Cfg;
use ttserver::cfg::Cfg as SlaveCfg;
use ttserver_def::*;
use ttutils::zlib;

pub(super) const CPU_TOTAL: i32 = 48;
pub(super) const MEM_TOTAL: i32 = 64 * 1024;
pub(super) const DISK_TOTAL: i32 = 1000 * 1024;

const IP_TEST: &str = "127.0.0.1";

lazy_static! {
    static ref PROXY_ADDR: String = [IP_TEST, ":19527"].concat();
    static ref CLI_SOCK: UdpSocket = pnk!(gen_sock(3));
    static ref SERV_ADDR: SocketAddr = pnk!(PROXY_ADDR.parse::<SocketAddr>());
    static ref OPS_MAP: HashMap<&'static str, u8> = map! {
        "register_client_id" => 0,
        "get_server_info" => 1,
        "get_env_list" => 2,
        "get_env_info" => 3,
        "add_env" => 4,
        "del_env" => 5,
        "update_env_lifetime" => 6,
        "update_env_kick_vm" => 7,
    };
}

pub(super) fn get_uid() -> u32 {
    getuid().as_raw()
}

// slave server[s] and proxy server
pub(super) fn start_proxy() {
    assert_eq!(
        0,
        get_uid(),
        "\x1b[31;1mMust be root to run this test!\x1b[0m"
    );

    let (proxy, server1, server2) = mock_cfg();

    start_slave(server1);
    start_slave(server2);

    thread::spawn(|| {
        pnk!(ttproxy::start(proxy));
    });

    thread::sleep(Duration::from_secs(3));
}

fn start_slave(cfg: SlaveCfg) {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            pnk!(ttserver::start(cfg));
            process::exit(1);
        }
        Ok(_) => {}
        e => {
            pnk!(e);
        }
    }
}

fn mock_cfg() -> (Cfg, SlaveCfg, SlaveCfg) {
    let s1 = [IP_TEST, ":29527"].concat();
    let s2 = [IP_TEST, ":39527"].concat();

    let cfgdb_path1 = format!("/tmp/{}", ts!());
    pnk!(fs::create_dir_all(&cfgdb_path1));

    let cfgdb_path2 = format!("/tmp/{}", 2 * ts!());
    pnk!(fs::create_dir_all(&cfgdb_path2));

    (
        Cfg {
            proxy_serv_at: PROXY_ADDR.to_owned(),
            server_addr_set: vct![
                pnk!(s1.parse::<SocketAddr>()),
                pnk!(s2.parse::<SocketAddr>())
            ],
            server_set: vct![s1.clone(), s2.clone()],
        },
        SlaveCfg {
            log_path: Some("/tmp/1_ttserver.log".to_owned()),
            serv_ip: IP_TEST.to_owned(),
            serv_at: s1,
            image_path: "/mnt/".to_owned(),
            cfgdb_path: cfgdb_path1,
            cpu_total: CPU_TOTAL,
            mem_total: MEM_TOTAL,
            disk_total: DISK_TOTAL,
        },
        SlaveCfg {
            log_path: Some("/tmp/2_ttserver.log".to_owned()),
            serv_ip: IP_TEST.to_owned(),
            serv_at: s2,
            image_path: "/mnt/".to_owned(),
            cfgdb_path: cfgdb_path2,
            cpu_total: CPU_TOTAL,
            mem_total: MEM_TOTAL,
            disk_total: DISK_TOTAL,
        },
    )
}

pub(super) type Sender<T> = fn(&str, Req<T>) -> Result<Resp>;

/// 发送请求信息, UDP 协议
pub(super) fn send_req<T: Serialize>(ops: &str, req: Req<T>) -> Result<Resp> {
    let ops_id = OPS_MAP
        .get(ops)
        .copied()
        .ok_or(eg!(format!("Unknown request: {}", ops)))?;

    let mut req_bytes = serde_json::to_vec(&req)
        .c(d!())
        .and_then(|req| zlib::encode(&req[..]).c(d!()))?;
    let mut body =
        format!("{id:>0width$}", id = ops_id, width = OPS_ID_LEN).into_bytes();
    body.append(&mut req_bytes);

    CLI_SOCK.send_to(&body, *SERV_ADDR).c(d!()).and_then(|_| {
        let mut buf = vct![0; 8 * 4096];
        let size = CLI_SOCK.recv(&mut buf).c(d!())?;

        zlib::decode(&buf[..size])
            .c(d!())
            .and_then(|resp_decompressed| {
                serde_json::from_slice(&resp_decompressed).c(d!())
            })
    })
}

/// 发送请求信息, HTTP/TCP 协议
pub(super) fn send_req_http<T: Serialize>(
    ops: &str,
    req: Req<T>,
) -> Result<Resp> {
    let url = format!("http://127.0.0.1:19527/{}", ops);
    let body = serde_json::to_vec(&req).c(d!())?;

    attohttpc::post(&url)
        .bytes(&body)
        .send()
        .c(d!())
        .and_then(|resp| resp.bytes().c(d!()))
        .and_then(|body| {
            zlib::decode(&body[..])
                .c(d!())
                .and_then(|resp_decompressed| {
                    serde_json::from_slice(&resp_decompressed).c(d!())
                })
        })
}

fn gen_sock(timeout: u64) -> Result<UdpSocket> {
    let mut addr;
    for port in (40_000 + ts!() % 10_000)..60_000 {
        addr = SocketAddr::from(([127, 0, 0, 1], port as u16));
        if let Ok(sock) = UdpSocket::bind(addr) {
            sock.set_read_timeout(Some(Duration::from_secs(timeout)))
                .c(d!())?;
            return Ok(sock);
        }
    }
    Err(eg!())
}
