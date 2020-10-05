//!
//! # Common utils
//!

use myutil::{err::*, *};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow, fs::File, ops::Deref, os::unix::io::RawFd, path::Path,
};

/// 服务端在回复 Resp 结构之前,
/// 发送4个字节长度的字符串数字给客户端,
/// 客户端据此接收 Resp 本体
pub const TRANS_META_WIDTH: usize = "1234".len();

/// 接收文件时以此为分界线,
/// - 低于此限, 按实际大小分配缓存区
/// - 超过此限, 循环利用此缓存区
#[cfg(feature = "server")]
pub(crate) const SIZE_16MB: usize = 16 * 1024 * 1024;

/// 传输方向
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum Direction {
    /// 从远程拉取文件
    Get,
    /// 推送文件至远程
    Push,
}

/// 文件传输的请求结构
#[derive(Debug, Deserialize, Serialize)]
pub struct TransReq<'a> {
    /// 传输方向
    pub drct: Direction,
    /// 客户端本地的文件路径,
    /// Push 时发送此文件至服务端,
    /// Get 时接收文件至本地的此路径,
    #[serde(skip)]
    pub local_file_path: &'a str,
    /// 服务端的文件路径
    pub remote_file_path: &'a str,
    /// 客户端发送的文件尺寸,
    /// 直接使用 local_file_path 的文件大小, 无需调方指定,
    /// Direction 为 Get 时忽略此项.
    pub(crate) file_size: usize,
}

impl<'a> TransReq<'a> {
    /// 创建实例的过程中,
    /// 会检查文件路径的有效性:
    /// - Push 时 local_file_path 必须存在并可读
    /// - Get 时 local_file_path 必须不存在, 防止覆盖已有文件
    pub fn new(
        drct: Direction,
        local_file_path: &'a str,
        remote_file_path: &'a str,
    ) -> Result<Self> {
        let file_size = if Direction::Push == drct {
            File::open(local_file_path)
                .c(d!())?
                .metadata()
                .c(d!())?
                .len() as usize
        } else if Path::new(local_file_path).exists() {
            return Err(eg!(format!(
                "File: `{}` already exists!",
                local_file_path
            )));
        } else {
            0
        };

        Ok(TransReq {
            drct,
            local_file_path,
            remote_file_path,
            file_size,
        })
    }
}

/// shell 执行命令的返回码,
/// 0 代表成功, 其它数字代表出错
pub type ShellCode = i32;

/// 服务端返回的元信息
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Resp<'a> {
    /// 服务端执行结果
    pub code: ShellCode,
    /// 服务端执行过程中产生的标准输出内容
    pub stdout: Cow<'a, str>,
    /// 服务端执行过程中产生的标准错误内容
    pub stderr: Cow<'a, str>,
    /// 服务端发送的文件尺寸,
    /// 直接使用 local_file_path 的文件大小, 无需调方指定,
    /// Direction 为 Get 时忽略此项.
    pub(crate) file_size: usize,
}

/// 管理 RawFd 的 lifetime, 确保及时关闭之
pub struct FileHdr(RawFd);

impl FileHdr {
    /// 创建新实例
    pub fn new(sock: RawFd) -> Self {
        FileHdr(sock)
    }
}

impl Deref for FileHdr {
    type Target = RawFd;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for FileHdr {
    fn drop(&mut self) {
        info_omit!(nix::unistd::close(self.0));
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
macro_rules! gen_sock {
    ($addr_family: tt, $sock_type: tt) => {{
        nix::sys::socket::socket(
            nix::sys::socket::AddressFamily::$addr_family,
            nix::sys::socket::SockType::$sock_type,
            nix::sys::socket::SockFlag::SOCK_CLOEXEC,
            None,
        )
        .map(FileHdr)
        .c(d!())
    }};
}

#[cfg(target_os = "macos")]
macro_rules! gen_sock {
    ($addr_family: tt, $sock_type: tt) => {{
        nix::sys::socket::socket(
            nix::sys::socket::AddressFamily::$addr_family,
            nix::sys::socket::SockType::$sock_type,
            nix::sys::socket::SockFlag::empty(),
            None,
        )
        .map(FileHdr)
        .c(d!())
    }};
}

/// 创建 UDP 套接字
#[inline(always)]
pub fn gen_udp_sock() -> Result<FileHdr> {
    gen_sock!(Inet, Datagram)
}

/// 创建 TCP 套接字
#[inline(always)]
pub fn gen_tcp_sock() -> Result<FileHdr> {
    gen_sock!(Inet, Stream)
}
