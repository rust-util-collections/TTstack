//!
//! # Commiuncation With Client
//!

use async_std::{
    net::{SocketAddr, UdpSocket},
    task,
};
use ruc::*;
use nix::sys::socket::{
    bind, sendto, setsockopt, socket, sockopt, AddressFamily, MsgFlags,
    SockFlag, SockType, UnixAddr, SockaddrStorage,
};
use serde::Serialize;
use std::{
    os::unix::io::{FromRawFd, RawFd, IntoRawFd, BorrowedFd},
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
/// Unix Socket that needs to receive messages must explicitly bind address;
/// If sent anonymously, unable to receive reply messages from the other party.
pub(crate) fn gen_uau_socket(addr: &[u8]) -> ruc::Result<(UdpSocket, UnixAddr)> {
    let owned_fd = socket(
        AddressFamily::Unix,
        SockType::Datagram,
        SockFlag::empty(),
        None,
    )
    .c(d!())?;
    
    let raw_fd = owned_fd.into_raw_fd();

    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(raw_fd) };
    setsockopt(&borrowed_fd, sockopt::ReuseAddr, &true).c(d!())?;
    setsockopt(&borrowed_fd, sockopt::ReusePort, &true).c(d!())?;

    let sa = UnixAddr::new(addr).c(d!())?;
    bind(raw_fd, &sa).c(d!())?;

    Ok((unsafe { UdpSocket::from_raw_fd(raw_fd) }, sa))
}

/// Send back success information
#[macro_export]
macro_rules! send_ok {
    ($uuid: expr, $msg: expr, $peeraddr: expr) => {
        $crate::util::send_back(
            *$crate::SOCK_UAU,
            $crate::util::gen_resp_ok($uuid, $msg),
            $peeraddr,
        )
    };
}

/// Generate reply body marking 'success'
pub(crate) fn gen_resp_ok(uuid: u64, msg: impl Serialize) -> Resp {
    Resp {
        uuid,
        status: RetStatus::Success,
        msg: info!(serde_json::to_vec(&msg)).unwrap_or_default(),
    }
}

/// Send back failure information
#[macro_export]
macro_rules! send_err {
    ($uuid: expr, $err: expr, $peeraddr: expr) => {{
        let log = genlog($err);
        $crate::util::send_back(
            *$crate::SOCK_UAU,
            $crate::util::gen_resp_err($uuid, &log),
            $peeraddr,
        )
        .c(d!(&log))
        .map_err(|e| { p(eg!(log)); e })
    }};
    // Errors generated directly at the top level, no longer forwarded internally
    (@$uuid: expr, $err: expr, $peeraddr: expr) => {{
        let log = genlog($err);
        $crate::util::send_out(
            &*$crate::SOCK,
            $crate::util::gen_resp_err($uuid, &log),
            $peeraddr,
        )
        .c(d!(&log))
        .map_err(|e| { p(eg!(log)); e })
    }};
}

/// Generate reply body marking 'error'
pub(crate) fn gen_resp_err(uuid: u64, msg: &str) -> Resp {
    Resp {
        uuid,
        status: RetStatus::Fail,
        msg: msg.as_bytes().to_vec(),
    }
}

/// Generate log message from error
pub(crate) fn genlog<E: std::fmt::Display>(err: E) -> String {
    format!("{}", err)
}

/// Print function for logging/debugging
pub(crate) fn p<T: std::fmt::Display>(msg: T) -> T {
    eprintln!("{}", msg);
    msg
}

/// Send information back to 'outbound hub'
#[inline(always)]
pub(crate) fn send_back(
    sock: RawFd,
    resp: Resp,
    peeraddr: SockaddrStorage,
) -> ruc::Result<()> {
    serde_json::to_vec(&resp)
        .c(d!())
        .and_then(|resp| zlib::encode(&resp[..]).c(d!()))
        .and_then(|resp_compressed| {
            sendto(sock, &resp_compressed, &peeraddr, MsgFlags::empty())
                .c(d!())
                .map(|_| ())
        })
}

/// Send information back to client
#[inline(always)]
pub(crate) fn send_out(
    sock: &'static UdpSocket,
    resp: Resp,
    peeraddr: SocketAddr,
) -> ruc::Result<()> {
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
