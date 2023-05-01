//!
//! # Design Model
//!
//! Design of the core infrastructure.
//!

use crate::{e, err::*};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs, mem,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Use `u64` to express an ID,
/// e.g. [EnvId](self::Env::id), [VmId](self::Vm::id) ...
pub type Id = u64;

/// Actual id will be alloced from 1,
/// 0 is the default value when needed.
pub const DEFAULT_ID: Id = 0;

/// ID alias for ENV.
pub type EnvId = Id;

/// ID alias for VM.
pub type VmId = Id;

/// Alias of user name.
pub type UserId = String;

/// Use `u16` to express a socket port.
pub type SockPort = u16;

/// Service ports within the host machine.
pub type PubSockPort = SockPort;

/// Service ports within the VM.
pub type InnerSockPort = SockPort;

/// Use `String` to express a network address,
/// in the the perspective of the client, see [NetAddr](self::Vm::addr).
pub type NetAddr = String;

/// Inner IP(v4) address of VM.
pub type IpAddr = [u8; 4];

/// MAC address of VM.
pub type MacAddr = [u8; 6];

/// Supported features of vm-engines.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum VmFeature {
    /// If snapshot is supported.
    Snapshot,
    /// If start/stop(aka pause) is supported.
    StartStop,
    /// If [Nat](self::NetKind::Nat) is supported.
    NatNetwork,
    /// If [Flatten](self::NetKind::Flatten) is supported.
    FlatNetwork,
}

/// Kind of network.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub enum NetKind {
    /// Like VxLan or Flannel(used in k8s).
    Flatten,
    /// Need firewall to do ports-forwarding.
    Forward,
    /// An alias of [Forward](self::NetKind::Forward).
    Nat,
}

impl Default for NetKind {
    fn default() -> Self {
        Self::Nat
    }
}

/// User -> [Env ...] -> [[Vm ...], ...]
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    /// Aka "user name".
    pub id: UserId,
    /// Optional password for a user.
    pub passwd: Option<String>,
    /// All envs belong to this user.
    pub env_set: HashSet<EnvId>,
}

/// The base unit of tt,
/// stands for a complete workspace for client.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Env {
    /// UUID of the ENV
    pub id: Id,
    /// Name of the ENV.
    pub name: Option<String>,
    /// The start timestamp of this ENV,
    /// can NOT be changed.
    pub start_timestamp: u64,
    /// The end timestamp of this ENV,
    /// permit user to change it .
    pub end_timestamp: u64,
    /// All [VmId](self::Vm::id)s under this ENV.
    pub vm_set: HashSet<Id>,
    /// Info about the state of ENV.
    pub state: EnvState,
}

/// Info about the state of ENV.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EnvState {
    /// Whether this ENV is stopped.
    pub is_stopped: bool,
    /// If true, all VMs of this ENV are denied to Internet.
    pub deny_outgoing: bool,
    /// The timestamp of the lastest manage-operation,
    /// such as 'stop/start/snapshot...'.
    ///
    /// This kind of operation can NOT be called frequently,
    /// because them may take a long time to complete.
    pub last_mgmt_timestamp: u64,
}

/// Infomations about a VM instance.
#[derive(Debug)]
pub struct Vm {
    /// UUID of this `VM`
    pub id: Id,
    /// Name of the `VM`.
    pub name: Option<String>,
    /// Created by which engine.
    pub engine: Arc<dyn VmEngine>,
    /// Template of `runtime_image`, that is,
    /// the runtime image is created based on the template.
    pub template: Arc<VmTemplate>,
    /// Runtime image of Vm.
    ///
    /// Use 'String' instead of 'PathBuf', because
    /// `runtime_image` may not be a regular file path,
    /// such as `ZFS` stroage.
    ///
    /// E.g. zfs/tt/[VmId](self::Vm::id)
    pub runtime_image: String,
    /// Network kind of this VM.
    pub net_kind: NetKind,
    /// SnapshotName => Snapshot
    pub snapshots: HashMap<String, Snapshot>,
    /// The latest cached config-file.
    pub latest_meta: Option<PathBuf>,
    /// Info about the state of VM.
    pub state: VmState,
    /// Info about the resource of VM.
    pub resource: VmResource,
    /// Usually an 'IP' or a 'domain url'.
    ///
    /// Only meaningful from the perspective of the client,
    /// to indicate how to connect to it from the client.
    ///
    /// This has different meanings with the
    /// [ip_addr](self::VmResource::ip_addr) in [VmResource](self::VmResource).
    pub addr: NetAddr,
    /// Features required by this vm.
    pub features: HashSet<VmFeature>,
}

/// Infomations about the template of VM,
/// or in other word, the base image of VM.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VmTemplate {
    /// Globally unique name,
    /// e.g. "github.com/ktmlm/alpine".
    pub name: String,
    /// Path which pointing to the template.
    /// May not be a regular file path, such as `ZFS`.
    pub path: String,
    /// Description of the template image, that is,
    /// the low-level infrastructure of the runtime image.
    pub memo: Option<String>,
    /// Engines compatible with this template, e.g. 'Qemu'.
    pub compatible_engines: HashSet<String>,
}

/// Info about the state of VM.
#[derive(Debug, Deserialize, Serialize)]
pub struct VmState {
    /// Whether has been stopped.
    pub during_stop: bool,
    /// Whether keep image NOT to be destroyed,
    /// when the life cycle of `Vm` ends.
    pub keep_image: bool,
    /// Whether generate a random uuid for this VM,
    /// if NOT, `machine-id` of the VM may be empty.
    pub rand_uuid: bool,
    /// VM can NOT connect to the addrs in this list.
    pub net_blacklist: HashSet<IpAddr>,
}

impl Default for VmState {
    fn default() -> Self {
        VmState {
            during_stop: true,
            keep_image: false,
            rand_uuid: true,
            net_blacklist: HashSet::new(),
        }
    }
}

/// Info about the resource of VM.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VmResource {
    /// Hardware, eg. cpu, mem, disk ...
    pub hw: Hardware,
    /// Inner IP address, e.g. '[10,0,0,2]',
    /// IP is generated from 'MAC address',
    /// use the last three fields of MAC.
    pub ip_addr: IpAddr,
    /// MAC address, e.g. '[0x82,0x17,0x0d,0x6a,0xbc,0x80]',
    /// used to generate the responding IP address.
    pub mac_addr: MacAddr,
    /// Ports allocation for NAT, that is:
    /// {Private Port within VM} => {Public Port within Host}.
    ///
    /// If the type of network is [Flatten](self::NetKind::Flatten),
    /// this field should be empty(and be ignored).
    pub port_map: HashMap<InnerSockPort, PubSockPort>,
}

/// Static resources, eg. cpu, mem.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Hardware {
    /// CPU number
    pub cpu_num: u16,
    /// Memory size in MB
    pub mem_size: u32,
    /// Disk size in MB
    pub disk_size: u32,
}

/// Snapshot management.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Snapshot {
    /// The name of snapshot.
    pub name: String,
    /// The data path of snapshot,
    /// May not be a regular file path, such as `ZFS`.
    pub path: String,
    /// The corresponding metadata to the snapshot.
    pub meta_path: PathBuf,
    /// If set this, snapshot will be cleaned when
    /// its life time exceeds this value, in seconds.
    pub life_time: Option<u64>,
}

impl Snapshot {
    /// Init a new snapshot instance without `life_time` limition.
    pub fn new(name: String, meta_path: PathBuf) -> Self {
        Self::newx(name, None, meta_path)
    }

    /// Init a new snapshot instance.
    pub fn newx(name: String, life_time: Option<u64>, meta_path: PathBuf) -> Self {
        Snapshot {
            name,
            life_time,
            path: String::new(),
            meta_path,
        }
    }
}

/// Common methods for each engine,
/// such as 'Firecracker', 'Qemu', 'Docker' ...
pub trait VmEngine: Send + Sync + Debug + Network + Storage {
    /// Will be called once during system starting.
    fn init() -> Result<Arc<dyn VmEngine>>
    where
        Self: Sized;

    /// Get the name of engine.
    fn name(&self) -> &str;

    /// Check if all wanted features can be supported.
    fn ok_features(&self, vm: &Vm) -> bool;

    /// Get all features supported by this engine.
    fn get_features(&self) -> &'static [VmFeature];

    /// Create the VM instance, and update necessary data of the `Vm`.
    fn create_vm(&self, vm: &mut Vm) -> Result<()> {
        self.create_image(vm)
            .c(d!(e!(ERR_TT_STORAGE_CREATE_IMAGE)))
            .and_then(|_| self.set_net(vm).c(d!(e!(ERR_TT_NET_SET_NET))))
    }

    /// Destroy the VM instance, and update necessary data of the `Vm`.
    fn destroy_vm(&self, vm: &mut Vm) -> Result<()> {
        self.destroy_image(vm)
            .c(d!(e!(ERR_TT_STORAGE_DESTROY_IMAGE)))
            .and_then(|_| self.unset_net(vm).c(d!(e!(ERR_TT_NET_UNSET_NET))))
    }

    /// Start a `stopped` VM.
    fn start_vm(&self, vm: &mut Vm) -> Result<()>;

    /// Stop(aka pause) a running VM,
    /// always cache meta in this function?
    fn stop_vm(&self, vm: &mut Vm) -> Result<()>;

    /// Replace the old `Vm` with a new one, apply all new configs.
    fn update_vm(&mut self, vm: Vm) -> Result<()>;

    /// Cache all infomations of the 'Vm' to disk.
    fn cache_meta(&self, vm: &Vm) -> Result<PathBuf>;

    /// Remove the cached config of `Vm`.
    fn clean_meta(&self, vm: &mut Vm, path: &Path) -> Result<()> {
        fs::remove_file(path).c(d!(e!(ERR_TT_SYS_IO))).map(|_| {
            if let Some(ref p) = vm.latest_meta {
                if p == path {
                    vm.latest_meta = None;
                }
            }
        })
    }

    /// Restruct a `Vm` from a cached config.
    fn restore_from_meta(&self, path: &Path) -> Result<Vm>;

    /// Add a snapshot for the runtime image:
    ///
    /// 1. stop the runtime instance
    /// 2. cache current meta-config
    /// 3. snapshot storage
    /// 4. restart the runtime instance
    fn create_snapshot(
        &self,
        vm: &mut Vm,
        name: &str,
        life_time: Option<u64>,
    ) -> Result<()> {
        self.stop_vm(vm)
            .c(d!(e!(ERR_TT_STOP_VM)))
            .and_then(|_| self.cache_meta(vm).c(d!(e!(ERR_TT_META_CREATE_CACHE))))
            .and_then(|_| {
                <Self as Storage>::create_snapshot(self, vm, name, life_time)
                    .c(d!(e!(ERR_TT_SNAPSHOT_CREATE)))
                    .map(|snap| {
                        vm.snapshots.insert(snap.path.clone(), snap);
                    })
            })
            .and_then(|_| self.start_vm(vm).c(d!(e!(ERR_TT_START_VM))))
    }

    /// Delete a snapshot of the runtime image:
    ///
    /// 1. remove the storage of snapshot
    /// 2. remove the cached-meta of snapshot
    fn destroy_snapshot(&self, vm: &mut Vm, name: &str) -> Result<()> {
        <Self as Storage>::destroy_snapshot(self, vm, name)
            .c(d!(e!(ERR_TT_SNAPSHOT_DESTROY)))
            .and_then(|snap| {
                self.clean_meta(vm, &snap.meta_path)
                    .c(d!(e!(ERR_TT_META_REMOVE_CACHE)))
            })
    }

    /// Revert to the state of this snapshot:
    ///
    /// 1. stop the runtime instance
    /// 3. relink runtime image to the one in snapshot
    /// 2. restore the responding [Vm](self::Vm) from cached-meta
    /// 4. restart the runtime instance
    fn apply_snapshot(&mut self, vm: &mut Vm, name: &str) -> Result<()> {
        self.stop_vm(vm)
            .c(d!())
            .and_then(|_| self.cache_meta(vm).c(d!(e!(ERR_TT_META_CREATE_CACHE))))
            .and_then(|_| {
                let snapshot = vm.snapshots.get(name).ok_or(eg!())?;
                let mut cached_vm = self
                    .restore_from_meta(&snapshot.meta_path)
                    .c(d!(e!(ERR_TT_META_RESTORE_CACHE)))?;
                <Self as Storage>::apply_snapshot(self, &mut cached_vm, snapshot)
                    .c(d!(e!(ERR_TT_SNAPSHOT_APPLY)))?;
                cached_vm.snapshots = mem::take(&mut vm.snapshots);
                self.update_vm(cached_vm).c(d!(e!(ERR_TT_UPDATE_VM)))
            })
            .and_then(|_| self.start_vm(vm).c(d!(e!(ERR_TT_START_VM))))
    }
}

/// This trait describes how to manage the network,
/// such as 'firewall rule' in the [NAT](self::NetKind::Nat) mode.
pub trait Network {
    /// Set network for the VM.
    fn set_net(&self, vm: &mut Vm) -> Result<()>;

    /// Unset network for the VM.
    fn unset_net(&self, vm: &mut Vm) -> Result<()>;

    /// Disable VM's active access to the Internet.
    fn deny_outgoing(&self, vm: &mut Vm) -> Result<()>;

    /// Enable VM's active access to the Internet.
    fn allow_outgoing(&self, vm: &mut Vm) -> Result<()>;

    /// There needs NOT a reponsponding `unset_` method,
    /// we can get an equal effect by clear the [net_blacklist](self::VmState::net_blacklist).
    fn set_blacklist(&self, vm: &mut Vm) -> Result<()>;
}

/// This trait describes how to manage the 'runtime image'.
pub trait Storage {
    /// Create a runtime image from it's template.
    fn create_image(&self, vm: &mut Vm) -> Result<()>;

    /// Destroy a runtime image.
    fn destroy_image(&self, vm: &mut Vm) -> Result<()>;

    /// Add a snapshot for the runtime image.
    fn create_snapshot(
        &self,
        _vm: &mut Vm,
        _name: &str,
        _life_time: Option<u64>,
    ) -> Result<Snapshot> {
        Ok(Default::default())
    }

    /// Delete a snapshot of the runtime image.
    fn destroy_snapshot(&self, _vm: &Vm, _snapshot_name: &str) -> Result<Snapshot> {
        Ok(Default::default())
    }

    /// Revert to the state of this snapshot.
    fn apply_snapshot(&self, _vm: &mut Vm, _snapshot: &Snapshot) -> Result<()> {
        Ok(())
    }
}
