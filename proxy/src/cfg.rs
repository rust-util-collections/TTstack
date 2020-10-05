//!
//! # Config Parse
//!

use std::net::SocketAddr;

/// 配置信息
#[derive(Debug)]
pub struct Cfg {
    /// Proxy 服务地址,
    /// eg: '0.0.0.0:19527'
    pub proxy_serv_at: String,
    /// Server 服务地址, SocketAddr 格式
    pub server_addr_set: Vec<SocketAddr>,
    /// Server 服务地址, 原始格式
    /// eg: '[ "127.0.0.1:9527", "10.10.10.101:9527" ]'
    pub server_set: Vec<String>,
}

pub(crate) fn register_cfg(cfg: Option<Cfg>) -> Option<&'static Cfg> {
    static mut CFG: Option<Cfg> = None;
    if cfg.is_some() {
        unsafe {
            CFG = cfg;
        }
    }
    unsafe { CFG.as_ref() }
}
