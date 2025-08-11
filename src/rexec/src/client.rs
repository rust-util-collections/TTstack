//!
//! # Client
//!

use crate::{common::*, sendfile::sendfile};
use ruc::*;
use nix::sys::socket::{self, MsgFlags, SockaddrIn};
use std::io::IoSlice;
use std::{
    fs::{self, File},
    io::Write,
    net::{SocketAddr, TcpStream},
    os::unix::io::IntoRawFd,
    time::Duration,
};

/// Send execution command request to remote,
/// - @ remote_addr: remote address, eg: "8.8.8.8:80"
/// - @ cmd: command to be executed, directly passed to shell for parsing and execution
/// - @ wait_timeout: maximum time to wait for remote response, default 10 seconds
pub fn req_exec<'a>(remote_addr: &'a str, cmd: &'a str) -> ruc::Result<Resp<'a>> {
    let socket = gen_udp_sock().c(d!())?;
    let sock = *socket;
    let peeraddr: SockaddrIn = remote_addr
        .parse::<SocketAddr>()
        .c(d!())?
        .into();
    let req = cmd.as_bytes();

    socket::sendto(sock, &req, &peeraddr, MsgFlags::empty())
        .c(d!())
        .and_then(|_| {
            let mut buf = vec![0; 4 * 4096];
            let recvd =
                socket::recv(sock, &mut buf, MsgFlags::empty()).c(d!())?;
            serde_json::from_slice::<Resp>(&buf[..recvd]).c(d!())
        })
}

/// Bidirectional file transfer
/// - @ remote_addr: remote address, eg: "8.8.8.8:80"
/// - @ request: request information, which will indicate transfer direction
/// - @ wait_timeout: maximum time to wait for remote response, default 10 seconds
pub fn req_transfer<'a>(
    remote_addr: &'a str,
    request: TransReq,
    wait_timeout: Option<u64>,
) -> ruc::Result<Resp<'a>> {
    let addr_std = remote_addr.parse::<SocketAddr>().c(d!())?;

    // Connection time at most 2 seconds
    let tcpstream =
        TcpStream::connect_timeout(&addr_std, Duration::from_secs(2))
            .c(d!())
            .and_then(|stream| {
                stream
                    .set_read_timeout(wait_timeout.map(Duration::from_secs))
                    .c(d!())
                    .map(|_| stream)
            })?;

    // Single read wait at most 3 seconds
    tcpstream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // Single write wait at most 3 seconds
    tcpstream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // Take over socket lifecycle
    let socket = FileHdr::new(tcpstream.into_raw_fd());
    let sock = *socket;

    let req = serde_json::to_vec(&request).c(d!())?;
    let meta =
        format!("{d:>0w$}", d = req.len(), w = TRANS_META_WIDTH).into_bytes();
    socket::sendmsg::<()>(
        sock,
        &[IoSlice::new(&meta), IoSlice::new(&req)],
        &[],
        MsgFlags::empty(),
        None,
    )
    .c(d!())?;

    // Transfer local file to remote
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

    // Store remote file locally
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
