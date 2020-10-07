use flate2::write::ZlibDecoder;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use nix::unistd::getuid;
use serde::Serialize;
use std::{
    collections::HashMap,
    io::Write,
    net::{SocketAddr, UdpSocket},
    thread,
    time::Duration,
};
use ttserver::cfg::Cfg;
use ttserver_def::*;

pub(super) const CPU_TOTAL: u32 = 48;
pub(super) const MEM_TOTAL: u32 = 64 * 1024;
pub(super) const DISK_TOTAL: u32 = 1000 * 1024;

lazy_static! {
    static ref CLI_SOCK: UdpSocket = pnk!(gen_sock(1));
    static ref SERV_ADDR: SocketAddr =
        pnk!("127.0.0.1:9527".parse::<SocketAddr>());
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

// 异步启动 Server
pub(super) fn start_server() {
    assert_eq!(
        0,
        get_uid(),
        "\x1b[31;1mMust be root to run this test!\x1b[0m"
    );

    thread::spawn(|| {
        pnk!(ttserver::start(mock_cfg()));
    });

    thread::sleep(Duration::from_secs(1));
}

fn mock_cfg() -> Cfg {
    Cfg {
        log_path: None,
        serv_ip: "127.0.0.1".to_owned(),
        serv_at: "127.0.0.1:9527".to_owned(),
        image_path: "/tmp/".to_owned(),
        cpu_total: CPU_TOTAL,
        mem_total: MEM_TOTAL,
        disk_total: DISK_TOTAL,
    }
}

/// 发送请求信息
pub(super) fn send_req<T: Serialize>(ops: &str, req: Req<T>) -> Result<Resp> {
    let ops_id = OPS_MAP
        .get(ops)
        .copied()
        .ok_or(eg!(format!("Unknown request: {}", ops)))?;

    let mut body =
        format!("{id:>0width$}", id = ops_id, width = OPS_ID_LEN).into_bytes();
    body.append(&mut serde_json::to_vec(&req).c(d!())?);

    CLI_SOCK.send_to(&body, *SERV_ADDR).c(d!()).and_then(|_| {
        let mut buf = vct![0; 8 * 4096];
        let size = CLI_SOCK.recv(&mut buf).c(d!())?;

        let mut d = ZlibDecoder::new(vct![]);
        d.write_all(&buf[..size])
            .c(d!())
            .and_then(|_| d.finish().c(d!()))
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
