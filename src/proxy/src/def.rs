//!
//! # Proxy Data Structure
//!
//! 选用 async req/resp 模型,
//! SlaveServer 的回复与 C/S 逻辑共用一套网络设施,
//! 请求发出后不做等待(包括异步等待和超时等待),后续根据 UUID 匹配处理.
//!

use myutil::*;
use nix::sys::socket::SockAddr;
use std::{collections::HashMap, mem, net::SocketAddr};
use ttserver_def::{Resp, UUID};

/// UNIX 时间戳
pub type TS = u64;

/// Proxy Bucket 索引
pub type IDX = usize;

/// Proxy 到 SlaveServer 的请求超时时间
pub const TIMEOUT_SECS: usize = 5;

/// 每秒轮询一次,
/// 将已超时的 bucket 中的数据整体丢弃,
/// 触发 Drop 机制, 在其中实现回复 Client 的逻辑
///
/// bucket 索引计算方式:
/// - `idx = ts!() % TIMEOUT_SECS`
#[derive(Default)]
pub struct Proxy {
    /// 查询 UUID 对应的 bucket
    pub idx_map: HashMap<UUID, IDX>,
    /// 按秒分割存储, 便于资源清理,
    /// 同一秒内产生的请求均位于同一 bucket 中
    pub buckets: [Bucket; TIMEOUT_SECS],
}

impl Proxy {
    /// 通常是每秒清理一次,
    /// 但定时任务不能保证与时间严格对齐,
    /// 比当前时间戳晚 5 秒以上的 bucket 都要清理
    pub fn clean_timeout(&mut self) {
        let ts_deadline = ts!() - TIMEOUT_SECS as u64;
        (0..TIMEOUT_SECS)
            .filter(|&i| self.buckets[i].ts < ts_deadline)
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|i| {
                mem::take(&mut self.buckets[i]).res.keys().for_each(|k| {
                    self.idx_map.remove(k);
                });
            })
    }
}

/// 轮询的基本单位
pub struct Bucket {
    /// 时间戳
    pub ts: TS,
    /// Slave 结果集
    pub res: HashMap<UUID, SlaveRes>,
}

impl Default for Bucket {
    fn default() -> Self {
        Bucket {
            ts: 0,
            res: HashMap::new(),
        }
    }
}

/// 请求信息发出之前,
/// 注册此结至 Proxy 中;
/// 除 msg 字段外,
/// 其余字段创建时预置
pub struct SlaveRes {
    /// Slave 回复的消息
    pub msg: HashMap<SocketAddr, Resp>,
    /// 还没收到回复的 Slave 数量,
    /// 每次收到回复减一, 减至 0 时丢弃该结构,
    /// 触发 Drop 机制处理数据并回复 Client 端
    pub num_to_wait: usize,
    /// 请求发起时间
    pub start_ts: u64,
    /// 全部回复或超时后,
    /// 调用此函数做最后的处理
    pub do_resp: fn(&mut SlaveRes),
    /// Client 的地址,
    /// do_resp 处理完后回复到此地址
    pub peeraddr: SockAddr,
    /// Clent 的 ReqId,
    /// 回复 Client 时会用到
    pub uuid: UUID,
}

/// 巧用 Drop
impl Drop for SlaveRes {
    fn drop(&mut self) {
        (self.do_resp)(self)
    }
}
