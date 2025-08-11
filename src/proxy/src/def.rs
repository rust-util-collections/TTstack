//!
//! # Proxy Data Structure
//!
//! Uses async req/resp model,
//! SlaveServer responses share the same network infrastructure with C/S logic,
//! No waiting after request is sent (including async waiting and timeout waiting), subsequent processing based on UUID matching.
//!

use ruc::*;
use nix::sys::socket::SockaddrStorage;
use std::{collections::HashMap, mem, net::SocketAddr};
use ttserver_def::{Resp, UUID};

/// UNIX timestamp
pub type TS = u64;

/// Proxy Bucket index
pub type IDX = usize;

/// Request timeout from Proxy to SlaveServer
pub const TIMEOUT_SECS: usize = 5;

/// Poll once per second,
/// Discard data in timed-out buckets entirely,
/// Trigger Drop mechanism, implement Client reply logic within it
///
/// bucket index calculation method:
/// - `idx = ts!() % TIMEOUT_SECS`
#[derive(Default)]
pub struct Proxy {
    /// Query bucket corresponding to UUID
    pub idx_map: HashMap<UUID, IDX>,
    /// Store by second division for easier resource cleanup,
    /// Requests generated within the same second are all in the same bucket
    pub buckets: [Bucket; TIMEOUT_SECS],
}

impl Proxy {
    /// Usually cleaned once per second,
    /// but scheduled tasks cannot guarantee strict time alignment,
    /// buckets more than 5 seconds later than current timestamp need to be cleaned
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

/// Basic unit of polling
pub struct Bucket {
    /// Timestamp
    pub ts: TS,
    /// Slave result set
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
