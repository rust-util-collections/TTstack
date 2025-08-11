//!
//! # Basic Type Definitions
//!

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// Default number of VM CPU cores
pub const CPU_DEFAULT: i32 = 2;
/// Default VM memory capacity, unit: MB
pub const MEM_DEFAULT: i32 = 1024;
/// Default VM disk capacity, unit: MB
pub const DISK_DEFAULT: i32 = 40 * 1024;

/// Cli ID
pub type CliId = String;
/// Cli ID as `&str`
pub type CliIdRef = str;
/// Env ID
pub type EnvId = String;
/// Env ID as `&str`
pub type EnvIdRef = str;

/// Uses the product of the last two segments of VM's MAC address, max value: 256 * 256
pub type VmId = i32;
/// process id
pub type Pid = u32;

/// VM default open port (sshd)
pub const SSH_PORT: u16 = 22;
/// VM default open port (ttrexec-daemon)
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
/// Port from VM internal perspective, such as standard ports 80, 443
pub type VmPort = Port;
/// Port from external perspective, such as NAT mapped ports 8080, 8443
pub type PubPort = Port;

/// May support more container engines in the future
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
#[allow(clippy::derivable_impls)]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Qemu
    }
}

#[cfg(target_os = "freebsd")]
#[allow(clippy::derivable_impls)]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Bhyve
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
#[allow(clippy::derivable_impls)]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Unknown
    }
}

/// Metadata for display purposes
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvMeta {
    /// Ensures global uniqueness
    pub id: EnvId,
    /// Start time cannot be changed once set
    pub start_timestamp: u64,
    /// End time can be changed to control VM lifecycle
    pub end_timestamp: u64,
    /// Number of VMs inside
    pub vm_cnt: usize,
    /// Whether in stopped state
    /// `tt env stop ...`
    pub is_stopped: bool,
}

/// Detailed information of environment instance
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvInfo {
    /// Ensures global uniqueness
    pub id: EnvId,
    /// Start time cannot be changed once set
    pub start_timestamp: u64,
    /// End time can be changed to control VM lifecycle
    pub end_timestamp: u64,
    /// Collection of all VMs under the same Env
    pub vm: BTreeMap<VmId, VmInfo>,
    /// Whether in stopped state
    /// `tt env stop ...`
    pub is_stopped: bool,
}

/// Use this structure to respond to client requests, preventing Drop actions
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmInfo {
    /// System name
    pub os: String,
    /// Number of CPUs
    pub cpu_num: i32,
    /// Unit: MB
    pub mem_size: i32,
    /// Unit: MB
    pub disk_size: i32,
    /// VM IP determined by VmId, using '10.10.x.x/8' network segment
    pub ip: Ipv4,
    /// Internal-external port mapping relationship for DNAT
    pub port_map: HashMap<VmPort, PubPort>,
}

/// Parameters provided by caller for VM creation
#[derive(Clone, Debug)]
pub struct VmCfg {
    /// System image path
    pub image_path: String,
    /// All VMs under the same Env have identical internal ports
    pub port_list: Vec<VmPort>,
    /// Type of virtual instance
    pub kind: VmKind,
    /// Number of CPUs
    pub cpu_num: Option<i32>,
    /// Unit: MB
    pub mem_size: Option<i32>,
    /// Unit: MB
    pub disk_size: Option<i32>,
    /// VM UUID randomization (unique)
    pub rand_uuid: bool,
}

/// Parameters provided by caller for VM creation, Proxy-specific
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmCfgProxy {
    /// Complete system image name
    pub os: String,
    /// All VMs under the same Env have identical internal ports
    pub port_list: Vec<VmPort>,
    /// Number of CPUs
    pub cpu_num: Option<i32>,
    /// Unit: MB
    pub mem_size: Option<i32>,
    /// Unit: MB
    pub disk_size: Option<i32>,
    /// VM UUID randomization (unique)
    pub rand_uuid: bool,
}
