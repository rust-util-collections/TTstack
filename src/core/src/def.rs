//!
//! # 基本类型定义
//!

#[cfg(not(feature = "testmock"))]
use crate::vmimg_path;
use crate::{nat, pause, resume, vm};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::atomic::{AtomicI32, AtomicU16, Ordering},
    sync::{Arc, Weak},
};
#[cfg(not(feature = "testmock"))]
use std::{thread, time};
pub(crate) use ttcore_def::*;

// VM 实例的生命周期最长 6 小时
const MAX_LIFE_TIME: u64 = 6 * 3600;
// `tt env start/stop ...` 最小间隔 20 秒
const MIN_START_STOP_ITV: u64 = 20;

/// eg: "QEMU:CentOS-7.2:default"
pub type OsName = String;
/// eg: "/dev/zvol/zroot/tt/QEMU:CentOS-7.2:default"
pub type ImagePath = String;

///////////////////
// Serv 相关定义 //
///////////////////

/// 服务定义
#[derive(Debug, Default)]
pub struct Serv {
    // 每个客户端对应的 Env 实例集合
    cli: Arc<RwLock<HashMap<CliId, HashMap<EnvId, Env>>>>,
    // Env 创建时添加, 销毁时删除
    env_id_inuse: Arc<Mutex<HashSet<EnvId>>>,
    // Vm 创建时添加, 销毁时删除
    vm_id_inuse: Arc<Mutex<HashSet<VmId>>>,
    // Vm 创建时添加, 销毁时删除
    pub_port_inuse: Arc<Mutex<HashSet<PubPort>>>,
    // 资源分配相关的统计数据
    resource: Arc<RwLock<Resource>>,
}

impl Serv {
    /// 创建服务实例
    #[inline(always)]
    pub fn new() -> Serv {
        Serv::default()
    }

    /// 设置可用的资源总量
    #[inline(always)]
    pub fn set_resource(&self, rsc: Resource) {
        *self.resource.write() =
            Resource::new(rsc.cpu_total, rsc.mem_total, rsc.disk_total);
    }

    /// 获取资源占用的统计数据
    #[inline(always)]
    pub fn get_resource(&self) -> Resource {
        *self.resource.read()
    }

    /// 清理过期的 Env
    pub fn clean_expired_env(&self) {
        let ts = ts!();

        let cli = self.cli.read();
        let expired = cli
            .iter()
            .map(|(cli_id, env)| {
                env.iter()
                    .filter(|(_, v)| v.end_timestamp < ts)
                    .map(move |(k, _)| (cli_id.clone(), k.clone()))
            })
            .flatten()
            .collect::<Vec<_>>();

        if !expired.is_empty() {
            drop(cli); // 换写锁
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

    /// 添加新的客户端
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

    /// 删除客户端并清理所有资源
    #[inline(always)]
    pub fn del_client(&self, id: &CliIdRef) {
        self.cli.write().remove(id);
    }

    /// 添加 Env, 若 CliId 不存在会自动创建
    #[inline(always)]
    pub fn register_env(&self, id: CliId, env: Env) -> Result<()> {
        let mut cli = self.cli.write();
        let env_set = cli.entry(id).or_insert(map! {});
        if env_set.get(&env.id).is_some() {
            Err(eg!("Env already exists!"))
        } else {
            env_set.insert(env.id.clone(), env);
            Ok(())
        }
    }

    /// 清除指定的 Env
    #[inline(always)]
    pub fn del_env(&self, cli_id: &CliIdRef, env_id: &EnvIdRef) {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            // drop 会自动清理资源
            env_set.remove(env_id);
        }
    }

    /// 暂停运行, 让出资源
    /// - 保留临时镜像和端口影射
    /// - 停止所有 VM 进程
    /// - 资源计数递减
    pub fn stop_env(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
    ) -> Result<()> {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let ts = ts!();
                if env.last_mgmt_ts + MIN_START_STOP_ITV > ts {
                    return Err(eg!(
                        "start/stop too frequency! wait 20 seconds, and try again."
                    ));
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
        }
        Ok(())
    }

    /// 恢复运行先前被 stop 的 ENV
    /// - 启动所有 VM 进程
    /// - 资源计数递增
    pub fn start_env(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
    ) -> Result<()> {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let ts = ts!();
                if env.last_mgmt_ts + MIN_START_STOP_ITV > ts {
                    return Err(eg!(
                        "start/stop too frequency! wait 20 seconds, and try again."
                    ));
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
        }
        Ok(())
    }

    /// 批量获取所有 Env 的概略信息
    #[inline(always)]
    pub fn get_env_meta(&self, cli_id: &CliIdRef) -> Vec<EnvMeta> {
        let get = |env: &HashMap<EnvId, Env>| {
            env.values().map(|i| i.as_meta()).collect::<Vec<_>>()
        };

        self.cli.read().get(cli_id).map(get).unwrap_or_default()
    }

    /// 获取全局 ENV 列表, 供 Proxy 使用
    #[inline(always)]
    pub fn get_env_meta_all(&self) -> Vec<EnvMeta> {
        self.cli
            .read()
            .values()
            .map(|env| env.values().map(|i| i.as_meta()))
            .flatten()
            .collect::<Vec<_>>()
    }

    /// 批量获取 Env 详细信息,
    /// 不能直接返回 Env 实体,
    /// 会触发 Drop 动作
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

    /// 更新指定 Env 的 lifetime
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
                env.update_life(lifetime, is_fucker).c(d!())
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }

    /// 删除指定 OS 前缀的 VM
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
                Ok(())
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }

    /// 更新指定 Env 的 lifetime
    #[inline(always)]
    pub fn update_env_hardware(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        cpu_mem_disk: (Option<u32>, Option<u32>, Option<u32>),
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
            } else {
                Err(eg!("Env NOT exists!"))
            }
        } else {
            Err(eg!("Client NOT exists!"))
        }
    }
}

/// 已分配的资源信息,
/// `*_used` 字段使用 u32 类型,
/// 防止统计数据时的加和运算溢出
#[derive(Clone, Copy, Debug, Default)]
pub struct Resource {
    /// Vm 数量
    pub vm_active: u32,
    /// Cpu 核心数
    pub cpu_total: u64,
    /// 已使用的 Cpu
    pub cpu_used: u32,
    /// 内存容量(MB)
    pub mem_total: u64,
    /// 已使用的内存(MB)
    pub mem_used: u32,
    /// 磁盘容量(MB)
    pub disk_total: u64,
    /// 已使用的磁盘(MB)
    pub disk_used: u32,
}

impl Resource {
    /// 设置资源限制时使用
    #[inline(always)]
    pub fn new(cpu_total: u64, mem_total: u64, disk_total: u64) -> Resource {
        let mut rsc = Resource::default();
        rsc.cpu_total = cpu_total;
        rsc.mem_total = mem_total;
        rsc.disk_total = disk_total;
        rsc
    }
}

//////////////////
// Env 相关定义 //
//////////////////

/// 描述一个环境实例
#[derive(Clone, Debug)]
pub struct Env {
    // 保证全局唯一
    id: EnvId,
    // 起始时间设定之后不允许变更
    start_timestamp: u64,
    // 结束时间可以变更, 用以控制 Vm 的生命周期
    end_timestamp: u64,
    // 标记该 ENV 是否处于 stop 状态
    is_stopped: bool,
    // 禁止外连网络
    outgoing_denied: bool,
    // 最近一次 stop 或 start 操作的时间,
    // 控制 `tt env start/stop <ENV>` 的执行频率,
    // 该类操作不能执行的太频繁, 会消耗性能并产生异常
    last_mgmt_ts: u64,
    // 同一 Env 下所有 Vm 集合
    vm: HashMap<VmId, Vm>,
    // 所属的 Serv 实例
    serv_belong_to: Weak<Serv>,
}

impl Env {
    /// 获取描述性的元信息
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

    /// 获取描述性的元信息
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

    /// 创建新的 Env 实例, 内部自动生成 ID
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
        })
    }

    /// 更新已有实例的生命周期
    #[inline(always)]
    pub fn update_life(&mut self, secs: u64, is_fucker: bool) -> Result<()> {
        if MAX_LIFE_TIME < secs && !is_fucker {
            return Err(eg!("Life time too long!"));
        }
        self.end_timestamp = self.start_timestamp + secs;
        Ok(())
    }

    /// 更新已有实例的硬件配置
    ///
    /// 除只更新端口的情况以外, 须检查:
    /// - ENV 必须处于 stop 状态
    /// - 系统资源足以支掌新的配置
    #[inline(always)]
    pub fn update_hardware(
        &mut self,
        cpu_num: Option<u32>,
        mem_size: Option<u32>,
        disk_size: Option<u32>,
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
                        self.vm.values().fold(vct![], |mut base, vm| {
                            vm.port_map.values().for_each(|port| {
                                // 清理端口 inuse 信息
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
                return Err(eg!("The fucking world is over!"));
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

    /// 批量创建 Vm 实例
    #[inline(always)]
    pub fn add_vm_set(&mut self, cfg_set: Vec<VmCfg>) -> Result<()> {
        let mut vm = vct![];

        // 检查可用资源
        self.check_resource(&cfg_set).c(d!())?;

        // 只做准备性工作, 不启动 VM 进程,
        // 具体的工作容, 在各系统平台上有不同的实现.
        //
        // 如:
        // - 创建 VM 运行时镜像
        // - 分配 VM 网络地址和端口影射
        // - ...
        //
        // 若出错返回, 相关资源会被`Drop`自动清理
        for cfg in cfg_set.into_iter() {
            vm.push(Vm::create_meta(&self.serv_belong_to, cfg)?);
        }

        // 若出错返回, 相关资源会被`Drop`自动清理
        Self::check_image(&vm).c(d!())?;

        // 准备工作成功完成后, 启动所有的 VM 进程;
        // 若出错返回, VM 进程及相关资源会被`Drop`自动清理
        for vm in vm.iter() {
            vm.start_vm().c(d!())?;
        }

        // 全部创建成功后再批量注册
        vm.into_iter().for_each(|vm| {
            self.vm.insert(vm.id(), vm);
        });

        Ok(())
    }

    // 检查实际的镜像文件是否已生成,
    // canonicalize() 会确保路径中涉及的所有环节均实际存在,
    #[cfg(not(feature = "testmock"))]
    fn check_image(vm: &[Vm]) -> Result<()> {
        let mut cnter = 0;
        let path_set = vm.iter().map(|i| vmimg_path(i)).collect::<Vec<_>>();

        // 每个`zfs clone`预期耗时100ms, 最少2s
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
                    eg!(@path_set.into_iter().filter(|i| i.canonicalize().is_err()).collect::<Vec<_>>()),
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

    // 检查可用资源是否充裕,
    // check 过程中将 u32 转换为 u64 进行计算, 避免溢出
    fn check_resource(&self, cfg_set: &[VmCfg]) -> Result<()> {
        if let Some(s) = self.serv_belong_to.upgrade() {
            let rsc;
            {
                rsc = *s.resource.read();
            }

            let (cpu, mem, disk) =
                cfg_set.iter().fold((0u64, 0, 0), |mut b, vm| {
                    b.0 += vm.cpu_num.unwrap_or(CPU_DEFAULT) as u64;
                    b.1 += vm.mem_size.unwrap_or(MEM_DEFAULT) as u64;
                    b.2 += vm.disk_size.unwrap_or(DISK_DEFAULT) as u64;
                    b
                });

            if rsc.cpu_used as u64 + cpu > rsc.cpu_total {
                return Err(eg!(format!(
                    "CPU resource busy: total {}, used {}, you want: {}",
                    rsc.cpu_total, rsc.cpu_used, cpu
                )));
            }

            if rsc.mem_used as u64 + mem > rsc.mem_total {
                return Err(eg!(format!(
                    "Memory resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.mem_total, rsc.mem_used, mem
                )));
            }

            if rsc.disk_used as u64 + disk > rsc.disk_total {
                return Err(eg!(format!(
                    "Disk resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.disk_total, rsc.disk_used, disk
                )));
            }
        } else {
            return Err(eg!("The fucking world is OVER!"));
        }

        Ok(())
    }

    // 检查可用资源是否充裕,
    // check 过程中将 u32 转换为 u64 进行计算, 避免溢出
    // - @cfg: (cpu_num, mem_size, disk_size)
    fn check_resource_and_set(&self, cfg: (u32, u32, u32)) -> Result<()> {
        if let Some(s) = self.serv_belong_to.upgrade() {
            let rsc = { *s.resource.read() };
            let vm_num = self.vm.len() as u64;

            let (cpu, mem, disk) =
                self.vm.values().fold((0u64, 0, 0), |mut b, vm| {
                    b.0 += vm.cpu_num as u64;
                    b.1 += vm.mem_size as u64;
                    b.2 += vm.disk_size as u64;
                    b
                });

            let (cpu_new, mem_new, disk_new) = (
                cfg.0 as u64 * vm_num,
                cfg.1 as u64 * vm_num,
                cfg.2 as u64 * vm_num,
            );

            if cpu_new > cpu
                && rsc.cpu_used as u64 + cpu_new - cpu > rsc.cpu_total
            {
                return Err(eg!(format!(
                    "CPU resource busy: total {}, used {}, you want: {}",
                    rsc.cpu_total, rsc.cpu_used, cpu_new
                )));
            }

            if mem_new > mem
                && rsc.mem_used as u64 + mem_new - mem > rsc.mem_total
            {
                return Err(eg!(format!(
                    "Memory resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.mem_total, rsc.mem_used, mem_new
                )));
            }

            if disk_new > disk
                && rsc.disk_used as u64 + disk_new - disk > rsc.disk_total
            {
                return Err(eg!(format!(
                    "Disk resource busy: total {} MB, used {} MB, you want: {} MB",
                    rsc.disk_total, rsc.disk_used, disk_new
                )));
            }

            // 确认无误后设置生效
            let mut r = s.resource.write();
            r.cpu_used =
                r.cpu_used + (cpu_new / vm_num) as u32 - (cpu / vm_num) as u32;
            r.mem_used =
                r.mem_used + (mem_new / vm_num) as u32 - (mem / vm_num) as u32;
            r.disk_used = r.disk_used + (disk_new / vm_num) as u32
                - (disk / vm_num) as u32;
        } else {
            return Err(eg!("The fucking world is OVER!"));
        }

        Ok(())
    }
}

// 清理资源占用
impl Drop for Env {
    fn drop(&mut self) {
        // 清理 Env 相关的 inuse 信息
        if let Some(s) = self.serv_belong_to.upgrade() {
            s.env_id_inuse.lock().remove(&self.id);
        }
    }
}

/////////////////
// Vm 配置定义 //
/////////////////

/// 用以与调用方交互
#[derive(Clone, Debug)]
pub struct VmCfg {
    /// 系统镜像路径
    pub image_path: String,
    /// 同一 Env 下所有 Vm 的内部端口都相同
    pub port_list: Vec<VmPort>,
    /// 虚拟实例的类型
    pub kind: VmKind,
    /// CPU 数量
    pub cpu_num: Option<u32>,
    /// 单位: MB
    pub mem_size: Option<u32>,
    /// 单位: MB
    pub disk_size: Option<u32>,
    /// VM uuid 随机化(唯一)
    pub rnd_uuid: bool,
}

/// 描述一个容器实例的信息
#[derive(Clone, Debug)]
pub struct Vm {
    /// Vm 镜像路径
    pub(crate) image_path: PathBuf,
    /// 虚拟实例的类型
    pub kind: VmKind,
    /// CPU 数量
    pub cpu_num: u32,
    /// 单位: MB
    pub mem_size: u32,
    /// 单位: MB
    pub disk_size: u32,

    // 所属的 Serv 实例
    serv_belong_to: Weak<Serv>,

    /// 实例 ID 与 IP 唯一对应
    pub(crate) id: VmId,
    /// Vm IP 由 VmId 决定, 使用'10.10.x.x/8'网段
    pub ip: Ipv4,
    /// 用于 DNAT 的内外端口影射关系,
    pub port_map: HashMap<VmPort, PubPort>,

    /// 是否处于暂停流程中
    pub during_stop: bool,

    /// VM 的 UUID 是否需要随机(唯一)生成
    pub rnd_uuid: bool,
}

impl Vm {
    #[inline(always)]
    pub(crate) fn as_info(&self) -> VmInfo {
        VmInfo {
            os: self
                .image_path
                .file_name()
                .map(|f| f.to_str())
                .flatten()
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
            id: -1,
            ip: Ipv4::default(),
            port_map: cfg.port_list.into_iter().fold(
                HashMap::new(),
                |mut acc, new| {
                    acc.insert(new, 0);
                    acc
                },
            ),
            during_stop: false,
            rnd_uuid: cfg.rnd_uuid,
        };

        // 创建之后须立即计数
        let cnt_it = |s: &Serv| {
            let mut rsc = s.resource.write();
            rsc.vm_active += 1;
            rsc.cpu_used += cpu_num;
            rsc.mem_used += mem_size;
            rsc.disk_used += disk_size;
        };

        if let Some(s) = serv.upgrade() {
            cnt_it(&s);
            res.alloc_resource(&s).c(d!()).map(|_| res)
        } else {
            Err(eg!())
        }
    }

    // 执行流程:
    //     1. 分配 VmId 并写入全局 inuse 中
    //     2. 依据 VmId 生成 Vm IP
    //     3. 分配对外通信的网络端口
    //     4. 设置 NAT 规则
    //     5. 创建 VM 运行时镜像
    // **此处不启动 VM 进程**
    #[inline(always)]
    fn alloc_resource(&mut self, serv: &Arc<Serv>) -> Result<()> {
        self.alloc_id(&serv)
            .c(d!())
            .map(|id| self.ip = Self::gen_ip(id))
            .and_then(|_| self.alloc_pub_port(&serv).c(d!()))
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

    // 分配 VmId 并写入全局 inuse 中
    #[inline(always)]
    fn alloc_id(&mut self, serv: &Arc<Serv>) -> Result<VmId> {
        const VM_ID_LIMIT: i32 = 0xffff;
        lazy_static! {
            static ref VM_ID: AtomicI32 = AtomicI32::new(0);
        }

        let vm_id = {
            let mut cnter = 0;
            let mut vmid_inuse = serv.vm_id_inuse.lock();
            loop {
                let id = VM_ID.fetch_add(1, Ordering::Relaxed) % VM_ID_LIMIT;
                if vmid_inuse.get(&id).is_none() {
                    vmid_inuse.insert(id);
                    self.id = id;
                    break id;
                }
                cnter += 1;
                if VM_ID_LIMIT < cnter {
                    return Err(eg!("The fucking world is over!!!"));
                }
            }
        };

        Ok(vm_id)
    }

    // 基于 VmId 生成 IP
    #[inline(always)]
    fn gen_ip(vm_id: VmId) -> Ipv4 {
        Ipv4::new(format!("10.10.{}.{}", vm_id / 256, vm_id % 256))
    }

    // 分配外部端口并写入全局 inuse 中
    fn alloc_pub_port(&mut self, serv: &Arc<Serv>) -> Result<()> {
        const PUB_PORT_LIMIT: u16 = 20000;
        const PUB_PORT_BASE: u16 = 40000;
        lazy_static! {
            static ref PUB_PORT: AtomicU16 = AtomicU16::new(PUB_PORT_BASE);
        }

        let mut cnter = 0;
        let mut v_cnter = self.port_map.len();
        let mut buf = vct![];
        while 0 < v_cnter {
            let mut port_inuse = serv.pub_port_inuse.lock();
            let port = PUB_PORT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                    Some(PUB_PORT_BASE + (1 + x) % PUB_PORT_LIMIT)
                })
                .map_err(|_| eg!(d!("The fucking world is over!!")))
                .c(d!())?;
            if port_inuse.get(&port).is_none() {
                port_inuse.insert(port);
                buf.push(port);
                v_cnter -= 1;
            }

            cnter += 1;
            if PUB_PORT_LIMIT < cnter {
                return Err(eg!("The fucking world is over!!!"));
            }
        }

        self.port_map.values_mut().zip(buf.into_iter()).for_each(
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
            // 清理 VmId inuse 信息
            s.vm_id_inuse.lock().remove(&self.id);

            // 清理资源统计数据, 若正处于暂停状态,
            // 其资源占用已经被减过一次了, 不能再减
            if !self.during_stop {
                let mut rsc = s.resource.write();
                rsc.vm_active -= 1;
                rsc.cpu_used -= self.cpu_num;
                rsc.mem_used -= self.mem_size;
                rsc.disk_used -= self.disk_size;
            }

            if !self.port_map.is_empty() {
                let mut pub_port = vct![];
                let mut inuse = s.pub_port_inuse.lock();
                self.port_map.values().for_each(|port| {
                    // 清理端口 inuse 信息
                    inuse.remove(port);
                    // 收集待清理端口
                    pub_port.push(*port);
                });

                // 清理 nat 规则
                info_omit!(nat::clean_rule(&[self]));
            }
        }

        vm::post_clean(self);
    }
}
