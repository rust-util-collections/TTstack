//!
//! # Environment Init.
//!

use crate::{hdr::server::refresh_os_info, util::asleep, CFG, POOL, SERV};
use myutil::{err::*, *};
use nix::unistd;
#[cfg(not(feature = "testmock"))]
use std::{fs, mem};

// 设置基本运行环境
#[inline(always)]
pub(super) fn setenv() -> Result<()> {
    unistd::chdir(CFG.image_path.as_str())
        .c(d!())
        .and_then(|_| log_init(CFG.log_path.as_deref()).c(d!()))
        .map(|_| set_total_resource())
}

///  设置可用的资源上限
fn set_total_resource() {
    SERV.set_resource(ttcore::Resource::new(
        CFG.cpu_total,
        CFG.mem_total,
        CFG.disk_total,
    ));
}

/// 每 15 秒执行一次定时任务
///     - 清理一次过期的 Vm
///     - 扫描刷新基础镜像信息
#[inline(always)]
pub(super) fn start_cron() {
    POOL.spawn_ok(async {
        loop {
            info_omit!(refresh_os_info().await);
            clean_expired_env().await;
            asleep(15).await;
        }
    });
}

// 清理过期的 Env,
// 只需请理 Env 自身即可,
// 其余附属数据会在 Drop 体制下被自动清理
#[inline(always)]
async fn clean_expired_env() {
    SERV.clean_expired_env()
}

// 输出日志至文件
#[cfg(not(feature = "testmock"))]
fn log_init(log_path: Option<&str>) -> Result<()> {
    const LOG_PATH: &str = "/tmp/ttserver.log";

    let path = log_path.unwrap_or(LOG_PATH);
    let open = || {
        fs::OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(path)
    };

    unistd::close(1)
        .c(d!())
        .and_then(|_| open().c(d!()))
        .map(mem::forget)
        .and_then(|_| unistd::close(2).c(d!()))
        .and_then(|_| open().c(d!()))
        .map(mem::forget)
}

// 输出日志至文件
#[cfg(feature = "testmock")]
fn log_init(_log_path: Option<&str>) -> Result<()> {
    Ok(())
}
