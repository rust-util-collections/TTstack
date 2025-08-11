//!
//! # Sendfile
//!
//! **注意**:
//!
//! - sendfile 单次最多发送 2G 内容, 需要循环
//! - sendfile 系统接口前两个参数的位置, 在 Linux 与 FreeBSD/MacOS 两类平台上是相反的
//!

use ruc::*;
use nix::sys::sendfile::sendfile as sf;
#[cfg(target_os = "freebsd")]
use nix::sys::sendfile::SfFlags;
use std::os::unix::io::{RawFd, BorrowedFd};

#[cfg(target_os = "linux")]
pub(crate) fn sendfile(
    file_fd: RawFd,
    sock_fd: RawFd,
    file_size: usize,
) -> ruc::Result<()> {
    let mut offset = 0;
    loop {
        let file_borrowed = unsafe { BorrowedFd::borrow_raw(file_fd) };
        let sock_borrowed = unsafe { BorrowedFd::borrow_raw(sock_fd) };
        let sendsiz =
            sf(sock_borrowed, file_borrowed, Some(&mut offset), file_size).c(d!())?;
        if 0 == sendsiz {
            break;
        }
    }
    Ok(())
}

#[cfg(target_os = "freebsd")]
pub(crate) fn sendfile(
    file_fd: RawFd,
    sock_fd: RawFd,
    file_size: usize,
) -> ruc::Result<()> {
    let mut offset = 0;
    loop {
        let file_borrowed = unsafe { BorrowedFd::borrow_raw(file_fd) };
        let sock_borrowed = unsafe { BorrowedFd::borrow_raw(sock_fd) };
        let (res, sendsiz) = sf(
            file_borrowed,
            sock_borrowed,
            offset,
            Some(file_size),
            None,
            None,
            SfFlags::empty(),
            16,
        );
        res.c(d!())?;
        if 0 == sendsiz {
            break;
        } else {
            offset += sendsiz;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn sendfile(
    file_fd: RawFd,
    sock_fd: RawFd,
    file_size: usize,
) -> ruc::Result<()> {
    let mut offset = 0;
    loop {
        let file_borrowed = unsafe { BorrowedFd::borrow_raw(file_fd) };
        let sock_borrowed = unsafe { BorrowedFd::borrow_raw(sock_fd) };
        let (res, sendsiz) =
            sf(file_borrowed, sock_borrowed, offset, Some(file_size as i64), None, None);
        res.c(d!())?;
        if 0 == sendsiz {
            break;
        } else {
            offset += sendsiz;
        }
    }
    Ok(())
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos"
)))]
pub(crate) fn sendfile(
    file_fd: RawFd,
    sock_fd: RawFd,
    file_size: usize,
) -> ruc::Result<()> {
    Err(eg!("Unsupported platform!"))
}
