//!
//! # 基本类型定义
//!

use myutil::{err::*, *};
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
    /// 客户端标识
    pub cli_id: CliId,
    /// 消息正文
    pub msg: T,
}

impl<T: Serialize> Req<T> {
    /// create a new instance
    pub fn new(uuid: u64, cli_id: CliId, msg: T) -> Self {
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
    pub vm_total: i32,
    pub cpu_total: i32,
    pub cpu_used: i32,
    pub mem_total: i32,
    pub mem_used: i32,
    pub disk_total: i32,
    pub disk_used: i32,
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
    pub life_time: Option<u64>,
    pub dup_each: Option<u32>,
    pub deny_outgoing: bool,

    /// 若此项为不空,
    /// 则优先使用此项的信息
    pub vmcfg: Option<Vec<VmCfgProxy>>,

    /// 若vmcfg字段为空值,
    /// 使用这些字段从头解析
    pub os_prefix: Vec<String>,
    pub cpu_num: Option<i32>,
    pub mem_size: Option<i32>,
    pub disk_size: Option<i32>,
    pub port_set: Vec<Port>,
    pub rand_uuid: bool,
}

impl ReqAddEnv {
    /// 自动添加 SSH/ttrexec 端口影射
    pub fn set_ssh_port(&mut self) {
        let set = |data: &mut Vec<VmPort>| {
            data.push(SSH_PORT);
            data.push(TTREXEC_PORT);
            data.sort_unstable();
            data.dedup();
        };

        if let Some(vc) = self.vmcfg.as_mut() {
            // requests from TTproxy
            vc.iter_mut().for_each(|cfg| {
                set(&mut cfg.port_list);
            });
        } else {
            // requests from TTclient
            set(&mut self.port_set);
        }
    }

    /// OS 前缀匹配不区分大小写
    pub fn set_os_lowercase(&mut self) {
        self.os_prefix
            .iter_mut()
            .for_each(|os| *os = os.to_lowercase());
    }

    /// 检查 dup 的数量是否超限
    pub fn check_dup(&self) -> Result<u32> {
        const DUP_MAX: u32 = 500;
        let dup_each = self.dup_each.unwrap_or(0);
        if DUP_MAX < dup_each {
            Err(eg!(format!(
                "the number of `dup` too large: {}(max {})",
                dup_each, DUP_MAX
            )))
        } else {
            Ok(dup_each)
        }
    }
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
    pub cpu_num: Option<i32>,
    pub mem_size: Option<i32>,
    pub disk_size: Option<i32>,
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
