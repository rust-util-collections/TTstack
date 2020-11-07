//!
//! # Commiuncation With Client
//!

use async_std::{
    net::{SocketAddr, UdpSocket},
    task,
};
use myutil::{err::*, *};
use nix::sys::socket::{
    bind, sendto, setsockopt, socket, sockopt, AddressFamily, MsgFlags,
    SockAddr, SockFlag, SockType, UnixAddr,
};
use serde::Serialize;
use std::{
    os::unix::io::{FromRawFd, RawFd},
    time::Duration,
};
use ttserver_def::*;
use ttutils::zlib;

/// UAU(uau), Unix(Unix Domain Socket) Abstract Udp
///
/// An abstract socket address is distinguished (from a pathname socket)
/// by the fact that sun_path[0] is a null byte ('\0').
/// The socket's address in this namespace is given
/// by the additional bytes in sun_path that are covered
/// by the specified length of the address structure.
/// Null bytes in the name have no special  significance.
/// The name has no connection with filesystem pathnames.
/// When the address of an abstract socket is returned,
/// the returned addrlen is greater than sizeof(sa_family_t) (i.e., greater than 2),
/// and the name of the socket is contained in the first (addrlen - sizeof(sa_family_t)) bytes of sun_path.
///
/// `man unix(7)` for more infomation.
///
/// NOTE:
/// 需要接收消息的 Unix Socket 必须显式绑定地址;
/// 若以匿名身份发送, 则无法收到对方的回复消息.
pub(crate) fn gen_uau_socket(addr: &[u8]) -> Result<(UdpSocket, SockAddr)> {
    let fd = socket(
        AddressFamily::Unix,
        SockType::Datagram,
        SockFlag::empty(),
        None,
    )
    .c(d!())?;

    setsockopt(fd, sockopt::ReuseAddr, &true).c(d!())?;
    setsockopt(fd, sockopt::ReusePort, &true).c(d!())?;

    let sa = SockAddr::Unix(UnixAddr::new_abstract(addr).c(d!())?);
    bind(fd, &sa).c(d!())?;

    Ok((unsafe { UdpSocket::from_raw_fd(fd) }, sa))
}

/// 回送成功信息
#[macro_export(crate)]
macro_rules! send_ok {
    ($uuid: expr, $msg: expr, $peeraddr: expr) => {
        $crate::util::send_back(
            *$crate::SOCK_UAU,
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
            *$crate::SOCK_UAU,
            $crate::util::gen_resp_err($uuid, &log),
            $peeraddr,
        )
        .c(d!(&log))
        .map(|_| p(eg!(log)))
    }};
    // 顶层直接产生的错误, 不再进行内部转发
    (@$uuid: expr, $err: expr, $peeraddr: expr) => {{
        let log = genlog($err);
        $crate::util::send_out(
            &*$crate::SOCK,
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

/// 回送信息至'外发中枢'
#[inline(always)]
pub(crate) fn send_back(
    sock: RawFd,
    resp: Resp,
    peeraddr: SockAddr,
) -> Result<()> {
    serde_json::to_vec(&resp)
        .c(d!())
        .and_then(|resp| zlib::encode(&resp[..]).c(d!()))
        .and_then(|resp_compressed| {
            sendto(sock, &resp_compressed, &peeraddr, MsgFlags::empty())
                .c(d!())
                .map(|_| ())
        })
}

/// 回送信息至客户端
#[inline(always)]
pub(crate) fn send_out(
    sock: &'static UdpSocket,
    resp: Resp,
    peeraddr: SocketAddr,
) -> Result<()> {
    serde_json::to_vec(&resp)
        .c(d!())
        .and_then(|resp| zlib::encode(&resp[..]).c(d!()))
        .map(|resp_compressed| {
            task::spawn(async move {
                info_omit!(sock.send_to(&resp_compressed, peeraddr).await);
            });
        })
}

#[inline(always)]
pub(crate) async fn asleep(secs: u64) {
    task::sleep(Duration::from_secs(secs)).await
}
