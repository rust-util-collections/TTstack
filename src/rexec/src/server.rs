//!
//! # Server
//!

use crate::{common::*, sendfile::sendfile};
use myutil::{err::*, *};
use nix::sys::{
    socket::{self, InetAddr, MsgFlags, SockAddr},
    uio::IoVec,
};
use std::{
    borrow::Cow,
    fs::{self, File},
    io::Write,
    net::SocketAddr,
    net::TcpStream,
    os::unix::io::{FromRawFd, IntoRawFd, RawFd},
    process, thread,
    time::Duration,
};

/// 响应 ExecReq 的 Daemon
pub fn serv_cmd(serv_addr: &str) -> Result<()> {
    let socket = gen_udp_sock().c(d!())?;
    let sock = *socket;

    set_reuse(sock).c(d!())?;

    let serv_addr = serv_addr
        .parse::<SocketAddr>()
        .c(d!())
        .map(|addr| SockAddr::new_inet(InetAddr::from_std(&addr)))?;

    socket::bind(sock, &serv_addr).c(d!())?;

    let mut cmd;
    let mut buf = vct![0; 8192];
    loop {
        if let Ok((size, Some(peeraddr))) =
            info!(socket::recvfrom(sock, &mut buf))
        {
            cmd = buf[..size].to_vec();
            thread::spawn(move || {
                info_omit!(run_cmd(cmd, sock, peeraddr));
            });
        }
    }
}

/// 执行命令
fn run_cmd(cmd: Vec<u8>, sock: RawFd, peeraddr: SockAddr) -> Result<()> {
    macro_rules! check_err {
        ($ops: expr) => {
            $ops.c(d!()).or_else(|e| {
                let log = genlog(e);
                let mut resp = Resp::default();
                resp.code = -1;
                resp.stderr = Cow::Borrowed(&log);
                socket::sendto(
                    sock,
                    &serde_json::to_vec(&resp).c(d!())?,
                    &peeraddr,
                    MsgFlags::empty(),
                )
                .c(d!())?;
                Err(eg!(log))
            })
        };
    }

    let mut resp = Resp::default();

    let stdout_path =
        format!("/tmp/.{}_{}_{}.stdout", peeraddr.to_str(), sock, ts!());
    let stderr_path =
        format!("/tmp/.{}_{}_{}.stderr", peeraddr.to_str(), sock, ts!());
    let cmd = String::from_utf8_lossy(&cmd).into_owned();
    let cmd = format!("({}) >{} 2>{}", cmd, stdout_path, stderr_path);

    let res = check_err!(
        process::Command::new("sh")
            .args(&["-c", &cmd])
            .spawn()
            .c(d!())
            .and_then(|mut child| child.wait().c(d!()))
    )?;

    if res.success() {
        let stdout = info!(fs::read(stdout_path)).unwrap_or_else(|_| {
            "Can NOT read stdout!".to_owned().into_bytes()
        });
        resp.code = 0;
        resp.stdout =
            Cow::Owned(String::from_utf8_lossy(&stdout).into_owned());
    } else {
        let stderr = info!(fs::read(stderr_path)).unwrap_or_else(|_| {
            "Can NOT read stderr!".to_owned().into_bytes()
        });
        // 无法获得退出码时, 返回 -1
        resp.code = res.code().unwrap_or(-1);
        resp.stderr =
            Cow::Owned(String::from_utf8_lossy(&stderr).into_owned());
    }

    check_err!(serde_json::to_vec(&resp))
        .and_then(|resp| {
            socket::sendto(sock, &resp, &peeraddr, MsgFlags::empty()).c(d!())
        })
        .map(|_| ())
}

/// 响应 TransReq 的 Daemon
pub fn serv_transfer(serv_addr: &str) -> Result<()> {
    let socket = gen_tcp_sock().c(d!())?;
    let sock = *socket;

    set_reuse(sock).c(d!())?;

    let serv_addr = serv_addr
        .parse::<SocketAddr>()
        .c(d!())
        .map(|addr| SockAddr::new_inet(InetAddr::from_std(&addr)))?;

    socket::bind(sock, &serv_addr).c(d!())?;
    socket::listen(sock, 8).c(d!())?;

    loop {
        if let Ok(fd) = info!(socket::accept(sock)) {
            thread::spawn(move || {
                info_omit!(do_serv_transfer(fd));
            });
        }
    }
}

fn do_serv_transfer(sock: RawFd) -> Result<()> {
    let stream = unsafe { TcpStream::from_raw_fd(sock) };

    // 单次读等待最多 3 秒
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // 单次写等待最多 3 秒
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .c(d!())?;

    // 接管生命周期, 确保及时关闭
    let socket = FileHdr::new(stream.into_raw_fd());
    let sock = *socket;

    macro_rules! send_back {
        ($resp: expr) => {
            let resp_bytes = serde_json::to_vec(&$resp).c(d!())?;
            let meta_bytes = format!(
                "{d:>0w$}",
                d = resp_bytes.len(),
                w = TRANS_META_WIDTH
            )
            .into_bytes();
            socket::sendmsg(
                sock,
                &[
                    IoVec::from_slice(&meta_bytes),
                    IoVec::from_slice(&resp_bytes),
                ],
                &[],
                MsgFlags::empty(),
                None,
            )
            .c(d!())?;
        };
    }

    macro_rules! check_err {
        ($ops: expr) => {
            $ops.c(d!()).or_else(|e| {
                let log = genlog(e);
                let mut resp = Resp::default();
                resp.code = -1;
                resp.stderr = Cow::Borrowed(&log);
                send_back!(resp);
                Err(eg!(log))
            })
        };
    }

    let mut meta_buf = [0; TRANS_META_WIDTH];
    let recvd =
        check_err!(socket::recv(sock, &mut meta_buf, MsgFlags::empty()))?;
    let req_size = check_err!(
        String::from_utf8_lossy(&meta_buf[..recvd]).parse::<usize>()
    )?;

    alt!(4096 < req_size, return Err(eg!("Maybe an attack!")));
    let mut req_buf = Vec::with_capacity(req_size);
    unsafe {
        req_buf.set_len(req_buf.capacity());
    }
    let recvd =
        check_err!(socket::recv(sock, &mut req_buf, MsgFlags::empty()))?;
    let req =
        check_err!(serde_json::from_slice::<TransReq>(&req_buf[..recvd]))?;

    match req.drct {
        Direction::Push => {
            let mut siz = req.file_size as usize;
            let file = check_err!(
                fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(req.remote_file_path)
            );
            let mut file = check_err!(file)?;
            let mut file_buf =
                Vec::with_capacity(alt!(siz > SIZE_16MB, SIZE_16MB, siz));
            unsafe {
                file_buf.set_len(file_buf.capacity());
            }
            let mut recvd;
            while siz > 0 {
                recvd = check_err!(socket::recv(
                    sock,
                    &mut file_buf,
                    MsgFlags::empty()
                ))?;
                if 0 == recvd {
                    return Err(eg!(format!(
                        "declared_size: {}, recvd: {}",
                        req.file_size,
                        req.file_size - siz
                    )));
                }
                check_err!(file.write(&file_buf[..recvd]))?;
                siz -= recvd;
            }

            // 已存储客户端上传的文件, 回复状态
            let mut resp = Resp::default();
            resp.stdout = Cow::Borrowed("Success!");
            send_back!(resp);
        }
        Direction::Get => {
            let file = check_err!(File::open(req.remote_file_path))?;

            // 先回复元信息
            let mut resp = Resp::default();
            resp.stdout = Cow::Borrowed("Request received! sending file...");
            resp.file_size = check_err!(file.metadata())?.len() as usize;
            send_back!(resp);

            // 然后再发送文件
            let fd_hdr = FileHdr::new(file.into_raw_fd());
            let fd = *fd_hdr;

            sendfile(fd, sock, resp.file_size).c(d!())?;
        }
    }

    Ok(())
}

/// reuse addr and port
fn set_reuse(sock: RawFd) -> Result<()> {
    socket::setsockopt(sock, socket::sockopt::ReuseAddr, &true)
        .c(d!())
        .and_then(|_| {
            socket::setsockopt(sock, socket::sockopt::ReusePort, &true).c(d!())
        })
}
