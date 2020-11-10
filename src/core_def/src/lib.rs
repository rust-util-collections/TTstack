//!
//! # 基本类型定义
//!

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// VM CPU 默认数量
pub const CPU_DEFAULT: i32 = 2;
/// VM MEM 默认容量, 单位: MB
pub const MEM_DEFAULT: i32 = 1024;
/// VM DISK 默认容量, 单位: MB
pub const DISK_DEFAULT: i32 = 40 * 1024;

/// Cli ID
pub type CliId = String;
/// Cli ID as `&str`
pub type CliIdRef = str;
/// Env ID
pub type EnvId = String;
/// Env ID as `&str`
pub type EnvIdRef = str;

/// 使用 Vm 的 MAC 地址的末尾两段的乘积, 最大值: 256 * 256
pub type VmId = i32;
/// process id
pub type Pid = u32;

/// VM 默认开放的端口(sshd)
pub const SSH_PORT: u16 = 22;
/// VM 默认开放的端口(ttrexec-daemon
pub const TTREXEC_PORT: u16 = 22000;

/// eg: 10.10.123.110
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct Ipv4 {
    addr: String,
}

impl Ipv4 {
    /// create a new one
    pub fn new(addr: String) -> Ipv4 {
        Ipv4 { addr }
    }

    /// convert to string
    pub fn as_str(&self) -> &str {
        self.addr.as_str()
    }
}

impl fmt::Display for Ipv4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.addr)
    }
}

/// eg: 22
pub type Port = u16;
/// Vm 内部视角的端口, 如 80、443 等标准端口
pub type VmPort = Port;
/// 外部视角的端口, 如 8080、8443 等 nat 出来的端口
pub type PubPort = Port;

/// 未来可能支持更多的容器引擎
/// - [Y] Qemu
/// - [Y] Bhyve
/// - [Y] Firecracker
/// - [N] Systemd Nspawn
/// - [N] Docker
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[allow(missing_docs)]
pub enum VmKind {
    Qemu,
    Bhyve,
    FireCracker,
    Unknown,
}

impl fmt::Display for VmKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmKind::Qemu => write!(f, "Qemu"),
            VmKind::Bhyve => write!(f, "Bhyve"),
            VmKind::FireCracker => write!(f, "FireCracker"),
            _ => write!(f, "Unknown"),
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Qemu
    }
}

#[cfg(target_os = "freebsd")]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Bhyve
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Unknown
    }
}

/// 元信息, 用于展示
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvMeta {
    /// 保证全局唯一
    pub id: EnvId,
    /// 起始时间设定之后不允许变更
    pub start_timestamp: u64,
    /// 结束时间可以变更, 用以控制 Vm 的生命周期
    pub end_timestamp: u64,
    /// 内部的 Vm 数量
    pub vm_cnt: usize,
    /// 是否处于停止状态
    /// `tt env stop ...`
    pub is_stopped: bool,
}

/// 环境实例的详细信息
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvInfo {
    /// 保证全局唯一
    pub id: EnvId,
    /// 起始时间设定之后不允许变更
    pub start_timestamp: u64,
    /// 结束时间可以变更, 用以控制 Vm 的生命周期
    pub end_timestamp: u64,
    /// 同一 Env 下所有 Vm 集合
    pub vm: BTreeMap<VmId, VmInfo>,
    /// 是否处于停止状态
    /// `tt env stop ...`
    pub is_stopped: bool,
}

/// 以此结构响应客户端请求, 防止触发 Drop 动作
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmInfo {
    /// 系统名称
    pub os: String,
    /// CPU 数量
    pub cpu_num: i32,
    /// 单位: MB
    pub mem_size: i32,
    /// 单位: MB
    pub disk_size: i32,
    /// Vm IP 由 VmId 决定, 使用'10.10.x.x/8'网段
    pub ip: Ipv4,
    /// 用于 DNAT 的内外端口影射关系,
    pub port_map: HashMap<VmPort, PubPort>,
}

/// 调用方提供的参数, 用以创建 VM
#[derive(Clone, Debug)]
pub struct VmCfg {
    /// 系统镜像路径
    pub image_path: String,
    /// 同一 Env 下所有 Vm 的内部端口都相同
    pub port_list: Vec<VmPort>,
    /// 虚拟实例的类型
    pub kind: VmKind,
    /// CPU 数量
    pub cpu_num: Option<i32>,
    /// 单位: MB
    pub mem_size: Option<i32>,
    /// 单位: MB
    pub disk_size: Option<i32>,
    /// VM uuid 随机化(唯一)
    pub rand_uuid: bool,
}

/// 调用方提供的参数, 用以创建 VM, Proxy 专用
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmCfgProxy {
    /// 完整的系统镜像名称
    pub os: String,
    /// 同一 Env 下所有 Vm 的内部端口都相同
    pub port_list: Vec<VmPort>,
    /// CPU 数量
    pub cpu_num: Option<i32>,
    /// 单位: MB
    pub mem_size: Option<i32>,
    /// 单位: MB
    pub disk_size: Option<i32>,
    /// VM uuid 随机化(唯一)
    pub rand_uuid: bool,
}
