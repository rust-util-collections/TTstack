//!
//! # Basic Type Definitions
//!

#[cfg(not(feature = "testmock"))]
use crate::vmimg_path;
use crate::{nat, pause, resume, vm};
use lazy_static::lazy_static;
use ruc::*;
use base64::{Engine as _, engine::general_purpose};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicI32, AtomicU16, Ordering},
    sync::{Arc, Weak},
};
#[cfg(not(feature = "testmock"))]
use std::{thread, time};
pub(crate) use ttcore_def::*;

// VM instance maximum lifetime is 6 hours
const MAX_LIFE_TIME: u64 = 6 * 3600;
// Minimum interval in seconds for `tt env start/stop ...`
const MIN_START_STOP_ITV: u64 = 20;
// Preset value before VmId allocation
const VM_PRESET_ID: i32 = -1;

const FUCK: &str = "The fucking world is over !!!";

/// eg: "QEMU:CentOS-7.2:default"
pub type OsName = String;
/// eg: "/dev/zvol/zroot/tt/QEMU:CentOS-7.2:default"
pub type ImagePath = String;

///////////////////
// Serv Related Definitions //
///////////////////

/// Service definition
#[derive(Debug, Default)]
pub struct Serv {
    // Collection of Env instances for each client
    cli: Arc<RwLock<HashMap<CliId, HashMap<EnvId, Env>>>>,
    // Added when Env is created, removed when destroyed
    env_id_inuse: Arc<Mutex<HashSet<EnvId>>>,
    // Added when VM is created, removed when destroyed
    vm_id_inuse: Arc<Mutex<HashSet<VmId>>>,
    // Added when VM is created, removed when destroyed
    pub_port_inuse: Arc<Mutex<HashSet<PubPort>>>,
    // Statistical data related to resource allocation
    resource: Arc<RwLock<Resource>>,
    /// configs of env[s]
    pub cfg_db: Arc<CfgDB>,
}

impl Serv {
    /// Create service instance
    #[inline(always)]
    pub fn new(cfgdb_path: &str) -> Serv {
        Serv {
            cfg_db: Arc::new(CfgDB::new(cfgdb_path)),
            ..Default::default()
        }
    }

    /// Set total available resources
    #[inline(always)]
    pub fn set_resource(&self, rsc: Resource) {
        *self.resource.write() =
            Resource::new(rsc.cpu_total, rsc.mem_total, rsc.disk_total);
    }

    /// Get resource usage statistics
    #[inline(always)]
    pub fn get_resource(&self) -> Resource {
        *self.resource.read()
    }

    /// Clean up expired Env
    pub fn clean_expired_env(&self) {
        let ts = ts!();

        let cli = self.cli.read();
        let expired = cli
            .iter()
            .flat_map(|(cli_id, env)| {
                env.iter()
                    .filter(|(_, v)| v.end_timestamp < ts)
                    .map(move |(k, _)| (cli_id.clone(), k.clone()))
            })
            .collect::<Vec<_>>();

        if !expired.is_empty() {
            drop(cli); // Switch to write lock
            let mut cli = self.cli.write();
            expired.iter().for_each(|(cli_id, k)| {
                cli.get_mut(cli_id.as_str())
                    .map(|env_set| env_set.remove(k));
            });
        }

        // clean zobmie process,
        // this will do nothing on freebsd.
        vm::zobmie_clean();
    }

    /// Add new client
    #[inline(always)]
    pub fn add_client(&self, id: CliId) -> Result<()> {
        let mut cli = self.cli.write();
        if cli.get(&id).is_some() {
            Err(eg!("Client already exists!"))
        } else {
            cli.insert(id, map! {});
            Ok(())
        }
    }

    /// Delete client and clean up all resources
    #[inline(always)]
    pub fn del_client(&self, id: &CliIdRef) {
        self.cli.write().remove(id);
    }

    /// !!! As the final step of creating new Env !!!
    /// Add Env, will auto-create if CliId doesn't exist
    #[inline(always)]
    pub fn register_env(&self, id: CliId, mut env: Env) -> Result<()> {
        let cli_id = id.clone();
        let mut cli = self.cli.write();
        let env_set = cli.entry(id).or_default();
        if env_set.get(&env.id).is_some() {
            Err(eg!("Env already exists!"))
        } else {
            // !!! Must be before cfg_db !!!
            // Created successfully, default to persistent cache;
            // Unless manually deleted by user or lifecycle expires
            env.vm.values_mut().for_each(|vm| {
                vm.image_cached = true;
            });

            self.cfg_db.write(&cli_id, &env).c(d!()).map(|_| {
                env.cli_belong_to = Some(cli_id);
                env_set.insert(env.id.clone(), env);
            })
        }
    }

    /// Remove specified Env
    #[inline(always)]
    pub fn del_env(&self, cli_id: &CliIdRef, env_id: &EnvIdRef) {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            // drop will automatically clean up resources
            if let Some(mut env) = env_set.remove(env_id) {
                // Mark instance images to be cleaned as deletable
                env.vm.values_mut().for_each(|vm| {
                    vm.image_cached = false;
                })
            }
        }
    }

    /// Pause execution, release resources
    /// - Keep temporary images and port mappings
    /// - Stop all VM processes
    /// - Decrement resource counters
    pub fn stop_env(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
    ) -> Result<()> {
        if let (Some(env_set), Some(env)) = (
            self.cli.write().get_mut(cli_id),
            self.cli
                .write()
                .get_mut(cli_id)
                .and_then(|env_set| env_set.get_mut(env_id)),
        ) {
            let ts = ts!();
            if env.last_mgmt_ts + MIN_START_STOP_ITV > ts {
                return Err(eg!(format!(
                    "Wait {} seconds, and try again!",
                    MIN_START_STOP_ITV
                )));
            }

            env.last_mgmt_ts = ts;
            for vm in env.vm.values_mut() {
                pause(vm.id()).c(d!()).map(|_| {
                    let mut rsc = self.resource.write();
                    rsc.vm_active -= 1;
                    rsc.cpu_used -= vm.cpu_num;
                    rsc.mem_used -= vm.mem_size;
                    rsc.disk_used -= vm.disk_size;
                    vm.during_stop = true;
                })?;
            }
            env.is_stopped = true;
        }
        Ok(())
    }

    /// Resume previously stopped ENV
    /// - Start all VM processes
    /// - Increment resource counters
    pub fn start_env(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
    ) -> Result<()> {
        if let (Some(env_set), Some(env)) = (
            self.cli.write().get_mut(cli_id),
            self.cli
                .write()
                .get_mut(cli_id)
                .and_then(|env_set| env_set.get_mut(env_id)),
        ) {
            let ts = ts!();
            if env.last_mgmt_ts + MIN_START_STOP_ITV > ts {
                return Err(eg!(format!(
                    "Wait {} seconds, and try again!",
                    MIN_START_STOP_ITV
                )));
            }

            env.last_mgmt_ts = ts;
            for vm in env.vm.values_mut() {
                resume(vm).c(d!()).map(|_| {
                    let mut rsc = self.resource.write();
                    rsc.vm_active += 1;
                    rsc.cpu_used += vm.cpu_num;
                    rsc.mem_used += vm.mem_size;
                    rsc.disk_used += vm.disk_size;
                    vm.during_stop = false;
                })?;
            }
            env.is_stopped = false;
        }
        Ok(())
    }

    /// Batch get summary information of all Env
    #[inline(always)]
    pub fn get_env_meta(&self, cli_id: &CliIdRef) -> Vec<EnvMeta> {
        let get = |env: &HashMap<EnvId, Env>| {
            env.values().map(|i| i.as_meta()).collect::<Vec<_>>()
        };

        self.cli.read().get(cli_id).map(get).unwrap_or_default()
    }

    /// Get global ENV list for Proxy usage
    #[inline(always)]
    pub fn get_env_meta_all(&self) -> Vec<EnvMeta> {
        self.cli
            .read()
            .values()
            .flat_map(|env| env.values().map(|i| i.as_meta()))
            .collect::<Vec<_>>()
    }

    /// Batch get detailed Env information,
    /// Cannot directly return Env entity,
    /// Would trigger Drop action
    #[inline(always)]
    pub fn get_env_detail(
        &self,
        cli_id: &CliIdRef,
        env_set: Vec<EnvId>,
    ) -> Vec<EnvInfo> {
        let get = |env: &HashMap<EnvId, Env>| {
            env.values()
                .filter(|v| env_set.iter().any(|vid| vid == &v.id))
                .map(|env| env.as_info())
                .collect::<Vec<_>>()
        };
        self.cli.read().get(cli_id).map(get).unwrap_or_default()
    }

    /// Update lifetime of specified Env
    #[inline(always)]
    pub fn update_env_life(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        lifetime: u64,
        is_fucker: bool,
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                env.update_life(lifetime, is_fucker)
                    .c(d!())
                    .and_then(|_| self.cfg_db.write(cli_id, env).c(d!()))
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }

    /// Delete VMs with specified OS prefix
    #[inline(always)]
    pub fn update_env_del_vm(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        vmid_set: &[VmId],
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                vmid_set.iter().for_each(|id| {
                    env.vm.remove(id);
                });
                self.cfg_db.write(cli_id, env).c(d!())
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }

    /// Update lifetime of specified Env
    #[inline(always)]
    pub fn update_env_hardware(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        cpu_mem_disk: (Option<i32>, Option<i32>, Option<i32>),
        vm_port: &[Port],
        deny_outgoing: Option<bool>,
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let (cpu_num, mem_size, disk_size) = cpu_mem_disk;
                env.update_hardware(
                    cpu_num,
                    mem_size,
                    disk_size,
                    vm_port,
                    deny_outgoing,
                )
                .c(d!())
                .and_then(|_| self.cfg_db.write(cli_id, env).c(d!()))
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }
}

/// Allocated resource information,
/// `*_used` fields use i32 type,
/// Prevent overflow in statistical data addition operations
#[derive(Clone, Copy, Debug, Default)]
pub struct Resource {
    /// Number of VMs
    pub vm_active: i32,
    /// Number of CPU cores
    pub cpu_total: i32,
    /// Used CPU
    pub cpu_used: i32,
    /// Memory capacity (MB)
    pub mem_total: i32,
    /// Used memory (MB)
    pub mem_used: i32,
    /// Disk capacity (MB)
    pub disk_total: i32,
    /// Used disk (MB)
    pub disk_used: i32,
}

impl Resource {
    /// Used when setting resource limits
    #[inline(always)]
    pub fn new(cpu_total: i32, mem_total: i32, disk_total: i32) -> Resource {
        Resource {
            cpu_total,
            mem_total,
            disk_total,
            ..Default::default()
        }
    }
}

//////////////////
// Env Related Definitions //
//////////////////

/// Describes an environment instance
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Env {
    /// Ensures global uniqueness
    pub id: EnvId,
    // Start time cannot be changed once set
    start_timestamp: u64,
    // End time can be changed to control VM lifecycle
    end_timestamp: u64,
    // Mark whether this ENV is in stopped state
    is_stopped: bool,
    /// Deny outgoing network connections
    pub outgoing_denied: bool,
    // Time of the most recent stop or start operation,
    // Controls the execution frequency of `tt env start/stop <ENV>`,
    // These operations cannot be executed too frequently, as it would consume performance and cause exceptions
    last_mgmt_ts: u64,
    /// Collection of all VMs under the same Env
    pub vm: HashMap<VmId, Vm>,
    // The Serv instance this belongs to
    #[serde(skip)]
    serv_belong_to: Weak<Serv>,
    /// The client this belongs to
    #[serde(skip)]
    pub cli_belong_to: Option<CliId>,
}

impl Env {
    /// Get descriptive metadata
    #[inline(always)]
    fn as_meta(&self) -> EnvMeta {
        EnvMeta {
            id: self.id.clone(),
            start_timestamp: self.start_timestamp,
            end_timestamp: self.end_timestamp,
            vm_cnt: self.vm.len(),
            is_stopped: self.is_stopped,
        }
    }

    /// Get descriptive metadata
    #[inline(always)]
    fn as_info(&self) -> EnvInfo {
        EnvInfo {
            id: self.id.clone(),
            start_timestamp: self.start_timestamp,
            end_timestamp: self.end_timestamp,
            vm: self.vm.iter().map(|(&k, v)| (k, v.as_info())).collect(),
            is_stopped: self.is_stopped,
        }
    }

    /// Load previously existing ENV instance
    pub fn load(mut self, serv: &Arc<Serv>) -> Result<Env> {
        let mut inuse = serv.env_id_inuse.lock();
        if inuse.get(&self.id).is_none() {
            inuse.insert(self.id.clone());
            drop(inuse);
        } else {
            return Err(eg!("Env already exists!"));
        }
        self.serv_belong_to = Arc::downgrade(serv);
        Ok(self)
    }

    /// Create new Env instance, automatically generate ID internally
    pub fn new(serv: &Arc<Serv>, id: &EnvIdRef) -> Result<Env> {
        let mut inuse = serv.env_id_inuse.lock();
        if inuse.get(id).is_none() {
            inuse.insert(id.to_owned());
            drop(inuse);
        } else {
            return Err(eg!("Env already exists!"));
        }

        Ok(Env {
            id: id.to_owned(),
            vm: HashMap::new(),
            start_timestamp: ts!(),
            end_timestamp: 3600 + ts!(),
            last_mgmt_ts: 0,
            is_stopped: false,
            outgoing_denied: false,
            serv_belong_to: Arc::downgrade(serv),
            cli_belong_to: None,
        })
    }

    /// Update lifecycle of existing instance
    #[inline(always)]
    pub fn update_life(&mut self, secs: u64, is_fucker: bool) -> Result<()> {
        if MAX_LIFE_TIME < secs && !is_fucker {
            return Err(eg!("Life time too long!"));
        }
        self.end_timestamp = self.start_timestamp + secs;
        Ok(())
    }

    /// Update hardware configuration of existing instance
    ///
    /// Except for port-only updates, must check:
    /// - ENV must be in stopped state
    /// - System resources are sufficient to support the new configuration
    #[inline(always)]
    pub fn update_hardware(
        &mut self,
        cpu_num: Option<i32>,
        mem_size: Option<i32>,
        disk_size: Option<i32>,
        vm_port: &[Port],
        deny_outgoing: Option<bool>,
    ) -> Result<()> {
        if [&cpu_num, &mem_size, &disk_size]
            .iter()
            .any(|i| i.is_some())
        {
            if !self.is_stopped {
                return Err(eg!(
                    "ENV must be stopped before updating it's hardware[s]."
                ));
            }

            let (cpu_new, mem_new, disk_new) =
                if let Some(vm) = self.vm.values().next() {
                    (
                        cpu_num.unwrap_or(vm.cpu_num),
                        mem_size.unwrap_or(vm.mem_size),
                        disk_size.unwrap_or(vm.disk_size),
                    )
                } else {
                    return Ok(());
                };

            self.check_resource_and_set((cpu_new, mem_new, disk_new))
                .c(d!())?;
            self.vm.values_mut().for_each(|vm| {
                vm.cpu_num = cpu_new;
                vm.mem_size = mem_new;
                vm.disk_size = disk_new;
            });
        }

        if !vm_port.is_empty() {
            let mut port = vm_port.to_vec();

            if let Some(s) = self.serv_belong_to.upgrade() {
                // 首先清理旧的端口影射
                {
                    let mut inuse = s.pub_port_inuse.lock();
                    let vm_set =
                        self.vm.values().fold(vec![], |mut base, vm| {
                            vm.port_map.values().for_each(|port| {
                                // Clean up port inuse information
                                inuse.remove(port);
                                // 收集 VM 集合
                                base.push(vm);
                            });
                            base
                        });
                    nat::clean_rule(vm_set.as_slice()).c(d!())?;
                }

                // 添加预置端口并去重
                port.push(SSH_PORT);
                port.push(TTREXEC_PORT);
                port.sort_unstable();
                port.dedup();

                // 然后生成新的影射关系
                for vm in self.vm.values_mut() {
                    vm.port_map = port.iter().map(|p| (*p, 0u16)).collect();
                    vm.alloc_pub_port(&s)
                        .c(d!())
                        .and_then(|_| nat::set_rule(vm).c(d!()))?;
                }
            } else {
                return Err(eg!(FUCK));
            }
        }

        // - 处于开放状态, 请求禁用外连网络, 执行策略变更
        // - 处于禁用状态, 请求开放外连网络, 执行策略变更
        // - 其它情况, 维持现状不变
        if let Some(deny) = deny_outgoing {
            let vm_set = self.vm.values().collect::<Vec<_>>();
            if deny && !self.outgoing_denied {
                nat::deny_outgoing(vm_set.as_slice()).c(d!())?;
                self.outgoing_denied = true;
            } else if !deny && self.outgoing_denied {
                nat::allow_outgoing(vm_set.as_slice()).c(d!())?;
                self.outgoing_denied = false;
            }
        }

        Ok(())
    }

    /// Batch create VM instances
    #[inline(always)]
    pub fn add_vm_set(&mut self, cfg_set: Vec<VmCfg>) -> Result<()> {
        self.add_vm_set_complex(cfg_set, vec![], false).c(d!())
    }

    /// Batch create or restore VM instances
    pub fn add_vm_set_complex(
        &mut self,
        cfg_set: Vec<VmCfg>,
        vm_set: Vec<Vm>,
        preload: bool,
    ) -> Result<()> {
        let mut vm = vec![];

        // Check available resources
        self.check_resource(&cfg_set).c(d!())?;

        // Only do preparatory work, do not start VM processes,
        // Specific work content has different implementations on various system platforms.
        //
        // Such as:
        // - Create VM runtime images
        // - Allocate VM network addresses and port mappings
        // - ...
        //
        // If error is returned, related resources will be automatically cleaned up by `Drop`
        if preload {
            for i in vm_set.into_iter() {
                vm.push(Vm::create_meta_from_cache(&self.serv_belong_to, i)?);
            }
        } else {
            for i in cfg_set.into_iter() {
                vm.push(Vm::create_meta(&self.serv_belong_to, i)?);
            }
        }

        // If error is returned, related resources will be automatically cleaned up by `Drop`
        Self::check_image(&vm).c(d!())?;

        // After preparatory work is successfully completed,
        // Start all VM processes that are not in 'during_stop' state;
        // If error is returned, all VM processes not in 'image_cached' state
        // and related resources will be automatically cleaned up by `Drop`
        for vm in vm.iter().filter(|i| !i.during_stop) {
            vm.start_vm().c(d!())?;
        }

        // Batch register after all are successfully created
        vm.into_iter().for_each(|vm| {
            self.vm.insert(vm.id(), vm);
        });

        if self.outgoing_denied {
            self.update_hardware(None, None, None, &[], Some(true))
                .c(d!())?;
        }

        Ok(())
    }

    // Check if actual image files have been generated,
    // canonicalize() ensures all components in the path actually exist,
    #[cfg(not(feature = "testmock"))]
    fn check_image(vm: &[Vm]) -> Result<()> {
        let mut cnter = 0;
        let path_set = vm.iter().map(vmimg_path).collect::<Vec<_>>();

        // Each `zfs clone` is expected to take 100ms, minimum 2s
        let mut timeout = (path_set.len() * 100) as u64;
        alt!(2000 > timeout, timeout = 2000);
        let timeout_unit = 200;
        let nr_limit = timeout / timeout_unit;

        while path_set
            .iter()
            .map(|i| i.canonicalize())
            .any(|i| i.is_err())
        {
            if nr_limit < cnter {
                return Err(
                    eg!(format!("Failed to canonicalize paths: {:?}", 
                        path_set.into_iter().filter(|i| i.canonicalize().is_err()).collect::<Vec<_>>())),
                );
            }

            cnter += 1;
            thread::sleep(time::Duration::from_millis(timeout_unit));
        }

        Ok(())
    }

    #[cfg(feature = "testmock")]
    fn check_image(_vm: &[Vm]) -> Result<()> {
        Ok(())
    }

    // Check if available resources are sufficient
    fn check_resource(&self, cfg_set: &[VmCfg]) -> Result<()> {
        if let Some(s) = self.serv_belong_to.upgrade() {
            let rsc = *s.resource.read();

            let (cpu, mem, disk) = cfg_set.iter().fold(
                (Some(0i32), Some(0i32), Some(0i32)),
                |b, vm| {
                    (
                        b.0.and_then(|i| {
                            i.checked_add(vm.cpu_num.unwrap_or(CPU_DEFAULT))
                        }),
                        b.1.and_then(|i| {
                            i.checked_add(vm.mem_size.unwrap_or(MEM_DEFAULT))
                        }),
                        b.2.and_then(|i| {
                            i.checked_add(vm.disk_size.unwrap_or(DISK_DEFAULT))
                        }),
                    )
                },
            );

            let (cpu, mem, disk) =
                if let (Some(c), Some(m), Some(d)) = (cpu, mem, disk) {
                    (c, m, d)
                } else {
                    return Err(eg!(FUCK));
                };

            if rsc.cpu_used.checked_add(cpu).ok_or(eg!(FUCK))? > rsc.cpu_total
            {
                return Err(eg!(format!(
                    "CPU resource busy: total {}, used {}, you want: {}",
                    rsc.cpu_total, rsc.cpu_used, cpu
                )));
            }

            if rsc.mem_used.checked_add(mem).ok_or(eg!(FUCK))? > rsc.mem_total
            {
                return Err(eg!(format!(
                    "Memory resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.mem_total, rsc.mem_used, mem
                )));
            }

            if rsc.disk_used.checked_add(disk).ok_or(eg!(FUCK))?
                > rsc.disk_total
            {
                return Err(eg!(format!(
                    "Disk resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.disk_total, rsc.disk_used, disk
                )));
            }
        } else {
            return Err(eg!(FUCK));
        }

        Ok(())
    }

    // Check if available resources are sufficient
    // - @cfg: (cpu_num, mem_size, disk_size)
    fn check_resource_and_set(&self, cfg: (i32, i32, i32)) -> Result<()> {
        if let Some(s) = self.serv_belong_to.upgrade() {
            let rsc = { *s.resource.read() };
            let vm_num = self.vm.len() as i32;

            let (cpu, mem, disk) =
                self.vm.values().fold((0, 0, 0), |mut b, vm| {
                    b.0 += vm.cpu_num;
                    b.1 += vm.mem_size;
                    b.2 += vm.disk_size;
                    b
                });

            let (cpu_new, mem_new, disk_new) =
                if let (Some(c), Some(m), Some(d)) = (
                    cfg.0.checked_mul(vm_num),
                    cfg.1.checked_mul(vm_num),
                    cfg.2.checked_mul(vm_num),
                ) {
                    (c, m, d)
                } else {
                    return Err(eg!(FUCK));
                };

            if cpu_new > cpu
                && rsc
                    .cpu_used
                    .checked_add(cpu_new)
                    .and_then(|i| i.checked_sub(cpu))
                    .ok_or(eg!(FUCK))?
                    > rsc.cpu_total
            {
                return Err(eg!(format!(
                    "CPU resource busy: total {}, used {}, you want: {}",
                    rsc.cpu_total, rsc.cpu_used, cpu_new
                )));
            }

            if mem_new > mem
                && rsc
                    .mem_used
                    .checked_add(mem_new)
                    .and_then(|i| i.checked_sub(mem))
                    .ok_or(eg!(FUCK))?
                    > rsc.mem_total
            {
                return Err(eg!(format!(
                    "Memory resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.mem_total, rsc.mem_used, mem_new
                )));
            }

            if disk_new > disk
                && rsc
                    .disk_used
                    .checked_add(disk_new)
                    .and_then(|i| i.checked_sub(disk))
                    .ok_or(eg!(FUCK))?
                    > rsc.disk_total
            {
                return Err(eg!(format!(
                    "Disk resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.disk_total, rsc.disk_used, disk_new
                )));
            }

            // 确认无误后设置生效
            let mut r = s.resource.write();
            r.cpu_used = r.cpu_used + (cpu_new / vm_num) - (cpu / vm_num);
            r.mem_used = r.mem_used + (mem_new / vm_num) - (mem / vm_num);
            r.disk_used = r.disk_used + (disk_new / vm_num) - (disk / vm_num);
        } else {
            return Err(eg!(FUCK));
        }

        Ok(())
    }
}

// Clean up resource usage
impl Drop for Env {
    fn drop(&mut self) {
        // Clean up Env-related inuse information
        if let Some(s) = self.serv_belong_to.upgrade() {
            s.env_id_inuse.lock().remove(&self.id);
            info_omit!(s.cfg_db.del(self));
        }
    }
}

/////////////////
// VM Configuration Definitions //
/////////////////

/// Information describing a container instance
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Vm {
    /// VM image path
    pub(crate) image_path: PathBuf,
    /// Type of virtual instance
    pub kind: VmKind,
    /// Number of CPUs
    pub cpu_num: i32,
    /// Unit: MB
    pub mem_size: i32,
    /// Unit: MB
    pub disk_size: i32,

    // The Serv instance this belongs to
    #[serde(skip)]
    serv_belong_to: Weak<Serv>,

    /// Instance ID uniquely corresponds to IP
    pub(crate) id: VmId,
    /// VM IP is determined by VmId, using '10.10.x.x/8' network segment
    pub ip: Ipv4,
    /// Internal and external port mapping relationship for DNAT,
    pub port_map: HashMap<VmPort, PubPort>,

    /// Whether it is in pause process
    pub during_stop: bool,

    /// Cache runtime image,
    /// i.e., not destroyed when process ends
    pub image_cached: bool,

    /// Whether VM UUID needs to be randomly (uniquely) generated
    pub rand_uuid: bool,
}

impl Vm {
    #[inline(always)]
    pub(crate) fn as_info(&self) -> VmInfo {
        VmInfo {
            os: self
                .image_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("Unknown")
                .to_owned(),
            cpu_num: self.cpu_num,
            mem_size: self.mem_size,
            disk_size: self.disk_size,
            ip: self.ip.clone(),
            port_map: self.port_map.clone(),
        }
    }

    pub(crate) fn create_meta(serv: &Weak<Serv>, cfg: VmCfg) -> Result<Vm> {
        let cpu_num = cfg.cpu_num.unwrap_or(CPU_DEFAULT);
        let mem_size = cfg.mem_size.unwrap_or(MEM_DEFAULT);
        let disk_size = cfg.disk_size.unwrap_or(DISK_DEFAULT);

        let mut res = Vm {
            image_path: PathBuf::from(cfg.image_path),
            kind: cfg.kind,
            cpu_num,
            mem_size,
            disk_size,
            serv_belong_to: Weak::clone(serv),
            id: VM_PRESET_ID,
            ip: Ipv4::default(),
            port_map: cfg.port_list.into_iter().fold(
                HashMap::new(),
                |mut acc, new| {
                    acc.insert(new, 0);
                    acc
                },
            ),
            during_stop: false,
            image_cached: false,
            rand_uuid: cfg.rand_uuid,
        };

        if let Some(s) = serv.upgrade() {
            res.alloc_resource(&s).c(d!()).map(|_| res)
        } else {
            Err(eg!())
        }
    }

    // Used when loading previously existing information (during service startup)
    #[inline(always)]
    pub(crate) fn create_meta_from_cache(
        serv: &Weak<Serv>,
        mut vm: Vm,
    ) -> Result<Vm> {
        vm.serv_belong_to = Weak::clone(serv);
        if let Some(s) = serv.upgrade() {
            vm.alloc_resource(&s).c(d!()).map(|_| vm)
        } else {
            Err(eg!())
        }
    }

    // Execution flow:
    //     1. Allocate VmId and write to global inuse
    //     2. Generate VM IP based on VmId
    //     3. Allocate network ports for external communication
    //     4. Set NAT rules
    //     5. Create VM runtime images
    // **VM processes are not started here**
    #[inline(always)]
    fn alloc_resource(&mut self, serv: &Arc<Serv>) -> Result<()> {
        // Pre-count before actually allocating resources
        let mut rsc = serv.resource.write();
        rsc.vm_active += 1;
        rsc.cpu_used += self.cpu_num;
        rsc.mem_used += self.mem_size;
        rsc.disk_used += self.disk_size;
        // Release lock
        drop(rsc);

        self.alloc_id(serv)
            .c(d!())
            .map(|id| self.ip = Self::gen_ip(id))
            .and_then(|_| self.alloc_pub_port(serv).c(d!()))
            .and_then(|_| nat::set_rule(self).c(d!()))
            .and_then(|_| self.pre_start().c(d!()))
    }

    #[inline(always)]
    fn pre_start(&self) -> Result<()> {
        vm::get_pre_starter(self)?(self).c(d!())
    }

    #[inline(always)]
    fn start_vm(&self) -> Result<()> {
        vm::start(self).c(d!())
    }

    // Allocate VmId and write to global inuse
    #[inline(always)]
    fn alloc_id(&mut self, serv: &Arc<Serv>) -> Result<VmId> {
        const VM_ID_LIMIT: i32 = 0xffff;
        lazy_static! {
            static ref VM_ID: AtomicI32 = AtomicI32::new(0);
        }

        let vm_id = {
            let mut vmid_inuse = serv.vm_id_inuse.lock();
            if VM_PRESET_ID == self.id {
                // Newly created VM during new service runtime
                let mut cnter = 0;
                loop {
                    let id =
                        VM_ID.fetch_add(1, Ordering::Relaxed) % VM_ID_LIMIT;
                    if vmid_inuse.get(&id).is_none() {
                        vmid_inuse.insert(id);
                        self.id = id;
                        break id;
                    }
                    cnter += 1;
                    if VM_ID_LIMIT < cnter {
                        return Err(eg!(FUCK));
                    }
                }
            } else if vmid_inuse.get(&self.id).is_none() {
                // Load previously existing VM during service initialization
                vmid_inuse.insert(self.id);
                self.id
            } else {
                // Duplicate VmId exists during service initialization, considered a serious error
                return Err(eg!(FUCK));
            }
        };

        Ok(vm_id)
    }

    // Generate IP based on VmId
    #[inline(always)]
    fn gen_ip(vm_id: VmId) -> Ipv4 {
        Ipv4::new(format!("10.10.{}.{}", vm_id / 256, vm_id % 256))
    }

    // Allocate external ports and write to global inuse
    fn alloc_pub_port(&mut self, serv: &Arc<Serv>) -> Result<()> {
        const PUB_PORT_LIMIT: u16 = 20000;
        const PUB_PORT_BASE: u16 = 40000;
        lazy_static! {
            static ref PUB_PORT: AtomicU16 = AtomicU16::new(PUB_PORT_BASE);
        }

        let mut cnter = 0;
        let mut v_cnter = self.port_map.len();
        let mut buf = vec![];
        while 0 < v_cnter {
            let mut port_inuse = serv.pub_port_inuse.lock();
            let port = PUB_PORT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                    Some(PUB_PORT_BASE + (1 + x) % PUB_PORT_LIMIT)
                })
                .map_err(|_| eg!(FUCK))
                .c(d!())?;
            if port_inuse.get(&port).is_none() {
                port_inuse.insert(port);
                buf.push(port);
                v_cnter -= 1;
            }

            cnter += 1;
            if PUB_PORT_LIMIT < cnter {
                return Err(eg!(FUCK));
            }
        }

        self.port_map.values_mut().zip(buf).for_each(
            |(p, port)| {
                *p = port;
            },
        );

        Ok(())
    }

    /// get VmId
    #[inline(always)]
    pub fn id(&self) -> VmId {
        self.id
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        if let Some(s) = self.serv_belong_to.upgrade() {
            // Clean up VmId inuse information
            s.vm_id_inuse.lock().remove(&self.id);

            // Clean up resource statistics, if currently in paused state,
            // its resource usage has already been decremented once, cannot be decremented again
            if !self.during_stop {
                let mut rsc = s.resource.write();
                rsc.vm_active -= 1;
                rsc.cpu_used -= self.cpu_num;
                rsc.mem_used -= self.mem_size;
                rsc.disk_used -= self.disk_size;
            }

            if !self.port_map.is_empty() {
                let mut pub_port = vec![];
                let mut inuse = s.pub_port_inuse.lock();
                self.port_map.values().for_each(|port| {
                    // Clean up port inuse information
                    inuse.remove(port);
                    // Collect ports to be cleaned
                    pub_port.push(*port);
                });

                // Clean up NAT rules
                info_omit!(nat::clean_rule(&[self]));
            }
        }

        vm::post_clean(self);
    }
}

////////////////
// Env Config DB //
////////////////

/// Manage Env information on disk
#[derive(Debug, Default)]
pub struct CfgDB {
    path: PathBuf,
}

impl CfgDB {
    /// create a new instance
    pub fn new(path: &str) -> CfgDB {
        let p = Path::new(path);
        assert!(p.is_dir());
        CfgDB {
            path: p.to_path_buf(),
        }
    }

    /// load all env[s] from disk
    pub fn read_all(&self) -> Result<HashMap<CliId, Vec<Env>>> {
        let get_cli_list = || -> Result<Vec<(CliId, PathBuf)>> {
            let mut list = vec![];
            for i in fs::read_dir(&self.path).c(d!())? {
                let entry = i.c(d!())?;
                let path = entry.path();
                if path.is_dir() {
                    let cli = path
                        .file_name()
                        .and_then(|p| p.to_str())
                        .ok_or(eg!())
                        .and_then(|cli| general_purpose::STANDARD.decode(cli.as_bytes()).c(d!()))
                        .map(|cli| {
                            String::from_utf8_lossy(&cli).into_owned()
                        })?;
                    list.push((cli, path.to_path_buf()));
                }
            }
            Ok(list)
        };

        let get_env_list = |cli_path: &Path| -> Result<Vec<Env>> {
            let mut list = vec![];
            for i in fs::read_dir(cli_path).c(d!())? {
                let entry = i.c(d!())?;
                let path = entry.path();
                if let Some(f) = path.file_name().and_then(|f| f.to_str()) {
                    if path.is_file() && f.ends_with(".json") {
                        fs::read(&path)
                            .c(d!())
                            .and_then(|data| {
                                serde_json::from_slice::<Env>(&data).c(d!())
                            })
                            .map(|env| {
                                if env.vm.values().any(|vm| vm.image_cached) {
                                    list.push(env);
                                } else {
                                    info_omit!(fs::remove_file(path));

                                    // When service process exits abnormally,
                                    // cleanup work is often not completed,
                                    // attempt to clean up again during new startup process
                                    env.vm.values().for_each(|i| {
                                        omit!(vm::post_clean(i));
                                    });
                                }
                            })?
                    }
                }
            }
            Ok(list)
        };

        let mut res = map! {};
        for (cli, path) in get_cli_list().c(d!())?.into_iter() {
            res.insert(cli, get_env_list(&path).c(d!())?);
        }

        Ok(res.into_iter().filter(|(_, v)| !v.is_empty()).collect())
    }

    /// write new config to disk
    pub fn write(&self, cli_id: &CliIdRef, env: &Env) -> Result<()> {
        serde_json::to_string_pretty(env).c(d!()).and_then(|cfg| {
            let mut cfgpath = self.path.clone();
            cfgpath.push(general_purpose::STANDARD.encode(cli_id));
            fs::create_dir_all(&cfgpath).c(d!())?;
            cfgpath.push(format!("{}.json", &env.id));
            fs::write(cfgpath, cfg).c(d!())
        })
    }

    /// delete config from disk
    pub fn del(&self, env: &Env) -> Result<()> {
        let mut cfgpath = self.path.clone();
        let cli = env
            .cli_belong_to
            .as_ref()
            .ok_or(eg!())
            .c(d!())
            .map(|cli| general_purpose::STANDARD.encode(cli))
            .map(|cli| String::from_utf8_lossy(cli.as_bytes()).into_owned())?;
        cfgpath.push(cli);
        cfgpath.push(format!("{}.json", &env.id));
        fs::remove_file(cfgpath).c(d!())
    }
}
