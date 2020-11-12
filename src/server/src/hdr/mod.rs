//!
//! # Network Service Handler
//!
//! Operations to deal with the requests from client.
//!

pub(crate) mod server;

use super::{send_err, send_ok, SERV};
use crate::{def::*, CFG};
use myutil::{err::*, *};
use serde::Deserialize;
use std::{
    mem,
    net::SocketAddr,
    ops::{Deref, DerefMut},
};
use ttcore::{vm_kind, Env};

type Ops = fn(SocketAddr, Vec<u8>) -> Result<()>;
include!("../../../server_def/src/included_ops_map.rs");

// 注册 Cli, 一般无需调用,
// 创建 Env 时若发现 Cli 不存在会自动创建之
fn register_client_id(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    let mut req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    SERV.add_client(mem::take(&mut req.cli_id))
        .c(d!())
        .and_then(|_| send_ok!(req.uuid, "Success!", peeraddr).c(d!()))
        .or_else(|e| send_err!(req.uuid, e, peeraddr).c(d!()))
}

/// 获取服务端的资源分配信息
fn get_server_info(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    let req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;

    let rsc = SERV.get_resource();
    let res = RespGetServerInfo {
        vm_total: rsc.vm_active,
        cpu_total: rsc.cpu_total,
        cpu_used: rsc.cpu_used,
        mem_total: rsc.mem_total,
        mem_used: rsc.mem_used,
        disk_total: rsc.disk_total,
        disk_used: rsc.disk_used,
        supported_list: server::OS_INFO.read().keys().cloned().collect(),
    };

    send_ok!(req.uuid, map! {CFG.serv_at.clone() => res}, peeraddr)
}

/// 获取服务端已存在的 Env 概略信息
fn get_env_list(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    let mut req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    let res = SERV.get_env_meta(&mem::take(&mut req.cli_id));
    send_ok!(req.uuid, map! {CFG.serv_at.clone() => res}, peeraddr).c(d!())
}

/// 获取服务端已存在的 Env 详细信息
fn get_env_info(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqGetEnvInfo,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let mut envinfo =
        SERV.get_env_detail(&mem::take(&mut req.cli_id), req.msg.env_set);

    // VM 的私有地址替换为服务器的地址
    envinfo.iter_mut().for_each(|env| {
        env.vm.values_mut().for_each(|vm| {
            vm.ip = Ipv4::new(CFG.serv_ip.clone());
        });
    });

    send_ok!(req.uuid, map! {CFG.serv_at.clone() => envinfo}, peeraddr).c(d!())
}

#[derive(Default)]
struct ReqAddEnvWrap(ReqAddEnv);

impl Deref for ReqAddEnvWrap {
    type Target = ReqAddEnv;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ReqAddEnvWrap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ReqAddEnvWrap {
    /// 根据请求参数生成 Env
    pub fn create_env(mut self) -> Result<Env> {
        self.set_ssh_port();
        let vmset = if let Some(vc) = self.vmcfg.take() {
            let mut res = vct![];
            let os_info = server::OS_INFO.read();
            for i in vc.into_iter() {
                res.push(VmCfg {
                    image_path: os_info
                        .get(&i.os)
                        .ok_or(eg!())
                        .c(d!())?
                        .to_owned(),
                    port_list: i.port_list,
                    kind: pnk!(vm_kind(&i.os)),
                    cpu_num: i.cpu_num,
                    mem_size: i.mem_size,
                    disk_size: i.disk_size,
                    rand_uuid: i.rand_uuid,
                });
            }
            res
        } else {
            self.set_os_lowercase();
            let me = &self;
            let dup_each = self.check_dup().c(d!())?;
            server::OS_INFO
                .read()
                .iter()
                .filter(|(os, _)| {
                    self.os_prefix.iter().any(|pre| os.starts_with(pre))
                })
                .map(|(os, img_path)| {
                    (0..(1 + dup_each)).map(move |_| VmCfg {
                        image_path: img_path.to_owned(),
                        port_list: me.port_set.clone(),
                        kind: pnk!(vm_kind(os)),
                        cpu_num: me.cpu_num,
                        mem_size: me.mem_size,
                        disk_size: me.disk_size,
                        rand_uuid: me.rand_uuid,
                    })
                })
                .flatten()
                .collect::<Vec<_>>()
        };

        if vmset.is_empty() {
            return Err(eg!("Nothing matches your OS-prefix[s] !"));
        }

        let mut env = Env::new(&SERV, &self.env_id).c(d!())?;
        env.update_life(self.life_time.unwrap_or(3600), false)
            .c(d!())?;

        env.outgoing_denied = self.deny_outgoing;
        env.add_vm_set(vmset).c(d!())?;

        Ok(env)
    }
}

/// 创建新的 Env
fn add_env(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqAddEnv,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let id = req.uuid;
    ReqAddEnvWrap(mem::take(&mut req.msg))
        .create_env()
        .c(d!())
        .and_then(|env| {
            SERV.register_env(mem::take(&mut req.cli_id), env).c(d!())
        })
        .and_then(|_| send_ok!(id, "Success!", peeraddr).c(d!()))
        .or_else(|e| send_err!(id, e, peeraddr).c(d!()))
}

/// 从已有 ENV 中踢出指定的 VM 实例
fn update_env_kick_vm(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvKickVm,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let cli_id = mem::take(&mut req.cli_id);
    SERV.get_env_detail(&cli_id, vct![mem::take(&mut req.msg.env_id)])
        .into_iter()
        .for_each(|ei| {
            info_omit!(
                SERV.update_env_del_vm(
                    &cli_id,
                    &ei.id,
                    ei.vm
                        .iter()
                        .filter(|(_, vm)| {
                            req.msg.os_prefix.iter().any(|prefix| {
                                vm.os
                                    .to_lowercase()
                                    .starts_with(&prefix.to_lowercase())
                            })
                        })
                        .map(|(&id, _)| id)
                        .chain(req.msg.vm_id.iter().copied())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            );
        });

    send_ok!(req.uuid, "Success!", peeraddr).c(d!())
}

/// 更新已有 Env 的生命周期
fn update_env_lifetime(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvLife,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    SERV.update_env_life(
        &mem::take(&mut req.cli_id),
        &req.msg.env_id,
        req.msg.life_time,
        req.msg.is_fucker,
    )
    .c(d!())
    .and_then(|_| send_ok!(req.uuid, "Success!", peeraddr).c(d!()))
    .or_else(|e| send_err!(req.uuid, e, peeraddr).c(d!()))
}

/// 删除 Env
fn del_env(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqDelEnv,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    SERV.del_env(&mem::take(&mut req.cli_id), &req.msg.env_id);

    send_ok!(req.uuid, "Success!", peeraddr).c(d!())
}

/// 获取服务端已存在的 Env 概略信息(全局)
fn get_env_list_all(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    let req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    let res = SERV.get_env_meta_all();
    send_ok!(req.uuid, map! {CFG.serv_at.clone() => res}, peeraddr).c(d!())
}

/// 暂停运行, 让出资源
/// - 资源计数递减
/// - 停止所有 VM 进程
/// - 保留临时镜像和端口影射
fn stop_env(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqStopEnv,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let id = req.uuid;

    SERV.stop_env(&mem::take(&mut req.cli_id), &req.msg.env_id)
        .c(d!())
        .and_then(|_| send_ok!(id, "Success!", peeraddr).c(d!()))
        .or_else(|e| send_err!(id, e, peeraddr).c(d!()))
}

/// 恢复运行先前被 stop 的 ENV
/// - 资源计数递增
/// - 启动所有 VM 进程
fn start_env(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqStartEnv,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let id = req.uuid;

    SERV.start_env(&mem::take(&mut req.cli_id), &req.msg.env_id)
        .c(d!())
        .and_then(|_| send_ok!(id, "Success!", peeraddr).c(d!()))
        .or_else(|e| send_err!(id, e, peeraddr).c(d!()))
}

/// 更新已有 Env 的资源配置
fn update_env_resource(peeraddr: SocketAddr, request: Vec<u8>) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvResource,
    }

    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    SERV.update_env_hardware(
        &mem::take(&mut req.cli_id),
        &req.msg.env_id,
        (
            req.msg.cpu_num.take(),
            req.msg.mem_size.take(),
            req.msg.disk_size.take(),
        ),
        &req.msg.vm_port,
        req.msg.deny_outgoing.take(),
    )
    .c(d!())
    .and_then(|_| send_ok!(req.uuid, "Success!", peeraddr).c(d!()))
    .or_else(|e| send_err!(req.uuid, e, peeraddr).c(d!()))
}
