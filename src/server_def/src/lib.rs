//!
//! # 基本类型定义
//!

use serde::{Deserialize, Serialize};
use std::fmt;
pub use ttcore_def::*;

/// ops_id 的字符长度, eg: "1234"
pub const OPS_ID_LEN: usize = 4;

/// 无法获取 uuid 时使用此默认 id
pub const DEFAULT_REQ_ID: u64 = std::u64::MAX;

/// uuid of req/resp
pub type UUID = u64;

/// - format: "<IP>:<PORT>"
/// - eg: "10.10.10.22:9527"
pub type ServerAddr = String;

/// Client 发送的信息
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Req<T: Serialize> {
    /// rpc uuid
    pub uuid: u64,
    /// 不指定则默认使用 IP
    pub cli_id: Option<CliId>,
    /// 消息正文
    pub msg: T,
}

impl<T: Serialize> Req<T> {
    /// create a new instance
    pub fn new(uuid: u64, msg: T) -> Self {
        Self::newx(uuid, None, msg)
    }

    /// create a new instance
    pub fn newx(uuid: u64, cli_id: Option<CliId>, msg: T) -> Self {
        Req { uuid, cli_id, msg }
    }
}

/// 服务端的执行结果
#[allow(missing_docs)]
#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd,
)]
pub enum RetStatus {
    Fail,
    Success,
}

impl fmt::Display for RetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            RetStatus::Fail => "Fail",
            RetStatus::Success => "Success",
        };

        write!(f, "{}", msg)
    }
}

/// 返回给 Client 的信息
#[derive(Debug, Deserialize, Serialize)]
pub struct Resp {
    /// rpc uuid
    pub uuid: u64,
    /// Fail? Success?
    pub status: RetStatus,
    /// 消息正文
    pub msg: Vec<u8>,
}

impl fmt::Display for Resp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "status: {}, msg: {}",
            self.status,
            String::from_utf8_lossy(&self.msg)
        )
    }
}

#[allow(missing_docs)]
#[derive(
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
pub struct RespGetServerInfo {
    pub vm_total: u32,
    pub cpu_total: u32,
    pub cpu_used: u32,
    pub mem_total: u32,
    pub mem_used: u32,
    pub disk_total: u32,
    pub disk_used: u32,
    pub supported_list: Vec<String>,
}

/// 直接使用 core 模块返回的结果
pub type RespGetEnvList = Vec<EnvMeta>;

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReqGetEnvInfo {
    pub env_set: Vec<EnvId>,
}

/// 直接使用 core 模块返回的结果
pub type RespGetEnvInfo = Vec<EnvInfo>;

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ReqAddEnv {
    pub env_id: EnvId,
    pub os_prefix: Vec<String>,
    pub life_time: Option<u64>,
    pub cpu_num: Option<u32>,
    pub mem_size: Option<u32>,
    pub disk_size: Option<u32>,
    pub port_set: Vec<Port>,
    pub dup_each: Option<u32>,
    pub deny_outgoing: bool,
}

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ReqStopEnv {
    pub env_id: EnvId,
}

/// 公开给 Cli 使用
pub type ReqStartEnv = ReqStopEnv;

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReqUpdateEnvLife {
    pub env_id: EnvId,
    pub life_time: u64,
    pub is_fucker: bool,
}

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReqUpdateEnvResource {
    pub env_id: EnvId,
    pub cpu_num: Option<u32>,
    pub mem_size: Option<u32>,
    pub disk_size: Option<u32>,
    pub vm_port: Vec<u16>,
    pub deny_outgoing: Option<bool>,
}

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReqDelEnv {
    pub env_id: EnvId,
}

/// 公开给 Cli 使用
#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReqUpdateEnvKickVm {
    pub env_id: EnvId,
    pub vm_id: Vec<VmId>,
    pub os_prefix: Vec<String>,
}
