//!
//! # Client
//!

use crate::{common::*, sendfile::sendfile};
use myutil::{err::*, *};
use nix::sys::{
    socket::{self, InetAddr, MsgFlags, SockAddr},
    uio::IoVec,
};
use std::{
    fs::{self, File},
    io::Write,
    net::{SocketAddr, TcpStream},
    os::unix::io::IntoRawFd,
    time::Duration,
};

/// 发送执行命令的请求至远端,
/// - @ remote_addr: 远端地址, eg: "8.8.8.8:80"
/// - @ cmd: 请求执行的命令, 直接交给 shell 解析并执行
/// - @ wait_timeout: 等待远程返回结果的最长时间, 默认 10 秒
pub fn req_exec<'a>(remote_addr: &'a str, cmd: &'a str) -> Result<Resp<'a>> {
    let socket = gen_udp_sock().c(d!())?;
    let sock = *socket;
    let peeraddr = remote_addr
        .parse::<SocketAddr>()
        .c(d!())
        .map(|addr| SockAddr::new_inet(InetAddr::from_std(&addr)))?;
    let req = cmd.as_bytes();

    socket::sendto(sock, &req, &peeraddr, MsgFlags::empty())
        .c(d!())
        .and_then(|_| {
            let mut buf = vct![0; 4 * 4096];
            let recvd =
                socket::recv(sock, &mut buf, MsgFlags::empty()).c(d!())?;
            serde_json::from_slice::<Resp>(&buf[..recvd]).c(d!())
        })
}

/// 双向互传文件
/// - @ remote_addr: 远端地址, eg: "8.8.8.8:80"
/// - @ request: 请求信息, 其中会标注传输方向
/// - @ wait_timeout: 等待远程返回结果的最长时间, 默认 10 秒
pub fn req_transfer<'a>(
    remote_addr: &'a str,
    request: TransReq,
    wait_timeout: Option<u64>,
) -> Result<Resp<'a>> {
    let addr_std = remote_addr.parse::<SocketAddr>().c(d!())?;

    // 连接时间最长 2 秒
    let tcpstream =
        TcpStream::connect_timeout(&addr_std, Duration::from_secs(2))
            .c(d!())
            .and_then(|stream| {
                stream
                    .set_read_timeout(wait_timeout.map(Duration::from_secs))
                    .c(d!())
                    .map(|_| stream)
            })?;

    // 单次读等待最多 3 秒
    tcpstream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // 单次写等待最多 3 秒
    tcpstream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // 接管 socket 的生命周期
    let socket = FileHdr::new(tcpstream.into_raw_fd());
    let sock = *socket;

    let req = serde_json::to_vec(&request).c(d!())?;
    let meta =
        format!("{d:>0w$}", d = req.len(), w = TRANS_META_WIDTH).into_bytes();
    socket::sendmsg(
        sock,
        &[IoVec::from_slice(&meta), IoVec::from_slice(&req)],
        &[],
        MsgFlags::empty(),
        None,
    )
    .c(d!())?;

    // 传送本地文件至远程
    if Direction::Push == request.drct {
        let fd = FileHdr::new(
            File::open(request.local_file_path).c(d!())?.into_raw_fd(),
        );
        let local_fd = *fd;

        sendfile(local_fd, sock, request.file_size).c(d!())?;
    }

    let mut buf = Vec::with_capacity(4 * 4096);

    unsafe {
        buf.set_len(TRANS_META_WIDTH);
    }
    let recvd = socket::recv(sock, &mut buf, MsgFlags::empty()).c(d!())?;
    let resp_size = String::from_utf8_lossy(&buf[..recvd])
        .parse::<usize>()
        .c(d!())?;

    if buf.capacity() < resp_size {
        return Err(eg!("The fucking world is over!"));
    } else {
        unsafe {
            buf.set_len(resp_size);
        }
    }
    let recvd = socket::recv(sock, &mut buf, MsgFlags::empty()).c(d!())?;
    let resp = serde_json::from_slice::<Resp>(&buf[..recvd]).c(d!())?;

    // 存储远程文件至本地
    if Direction::Get == request.drct && 0 == resp.code {
        let mut siz = resp.file_size as usize;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(request.local_file_path)
            .c(d!())?;
        let mut recvd;
        unsafe {
            buf.set_len(buf.capacity());
        }

        while siz > 0 {
            recvd = socket::recv(sock, &mut buf, MsgFlags::empty()).c(d!())?;
            if 0 == recvd {
                return Err(eg!(format!(
                    "declared_size: {}, recvd: {}",
                    resp.file_size,
                    resp.file_size - siz
                )));
            }
            file.write(&buf[..recvd]).c(d!())?;
            siz -= recvd;
        }
    }

    Ok(resp)
}
