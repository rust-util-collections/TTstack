//!
//! # Network Service Handler
//!
//! Operations to deal with the requests from client.
//!
//! 与 Server 同名接口的对外表现完全一致.
//!

mod add_env;
pub(crate) mod sync;

use crate::{def::*, send_err, send_ok, CFG, PROXY, SOCK_MID};
use add_env::add_env;
use async_std::{net::SocketAddr, task};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use nix::sys::socket::SockAddr;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use std::{
    collections::HashMap,
    mem,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use ttserver_def::*;
use ttutils::zlib;

lazy_static! {
    static ref ENV_MAP: Arc<RwLock<HashMap<EnvId, Vec<SocketAddr>>>> =
        Arc::new(RwLock::new(map! {}));
    static ref SLAVE_INFO: Arc<RwLock<HashMap<SocketAddr, RespGetServerInfo>>> =
        Arc::new(RwLock::new(map! {}));
}

type Ops = fn(usize, SockAddr, Vec<u8>) -> Result<()>;
include!("../../../server_def/src/included_ops_map.rs");

/// 将客户端的请求,
/// 分发至后台的各个 Slave Server.
#[macro_export(crate)]
macro_rules! fwd_to_slave {
    (@@@$ops_id: expr, $req: expr, $peeraddr: expr, $cb: tt, $addr_set: expr) => {{
        let num_to_wait = $addr_set.len();
        let proxy_uuid = gen_proxy_uuid();

        register_resp_hdr(num_to_wait, $cb, $peeraddr, $req.uuid, proxy_uuid);

        let cli_id = mem::take(&mut $req.cli_id);
        send_req_to_slave($ops_id, Req::new(proxy_uuid, cli_id, mem::take(&mut $req.msg)), $addr_set)
            .c(d!())
    }};
    (@@$ops_id: expr, $req: expr, $peeraddr: expr, $cb: tt, $addr_set: expr) => {{
        let mut req = $req;
        fwd_to_slave!(@@@$ops_id, req, $peeraddr, $cb, $addr_set)
            .or_else(|e| send_err!(req.uuid, e, $peeraddr))
    }};
    (@$ops_id: expr, $orig_req: expr, $req_kind: ty, $peeraddr: expr, $cb: tt) => {{
        let req = serde_json::from_slice::<$req_kind>(&$orig_req).c(d!())?;
        let addr_set = if let Some(set) = ENV_MAP.read().get(&req.msg.env_id) {
            set.clone()
        } else {
            return send_err!(req.uuid, eg!("ENV not exists!"), $peeraddr);
        };
        fwd_to_slave!(@@$ops_id, req, $peeraddr, $cb, &addr_set)
    }};
    ($ops_id: expr, $orig_req: expr, $req_kind: ty, $peeraddr: expr) => {{
        fwd_to_slave!(@$ops_id, $orig_req, $req_kind, $peeraddr, resp_cb_simple)
    }};
}

/// 注册 Cli, 一般无需调用,
/// 创建 Env 时若发现 Cli 不存在会自动创建之,
/// 此接口在 Proxy 中实现为"什么都不做, 直接返回成功".
pub(crate) fn register_client_id(
    _ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    let req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    let resp = Resp {
        uuid: req.uuid,
        status: RetStatus::Success,
        msg: vct![],
    };
    send_ok!(req.uuid, resp, peeraddr)
}

/// 获取服务端的资源分配信息,
/// 直接从定时任务的结果中提取, 不做实时请求.
pub(crate) fn get_server_info(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    // 汇聚各 Slave 的信息
    fn cb(r: &mut SlaveRes) {
        info_omit!(resp_cb_merge::<RespGetServerInfo>(r));
    }

    let req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    let addr_set = CFG.server_addr_set.clone();

    fwd_to_slave!(@@ops_id, req, peeraddr, cb, &addr_set)
}

/// 获取服务端已存在的 Env 概略信息
pub(crate) fn get_env_list(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    // 汇聚各 Slave 的信息
    fn cb(r: &mut SlaveRes) {
        info_omit!(resp_cb_merge::<RespGetEnvList>(r));
    }

    let req = serde_json::from_slice::<Req<&str>>(&request).c(d!())?;
    let addr_set = CFG.server_addr_set.clone();

    fwd_to_slave!(@@ops_id, req, peeraddr, cb, &addr_set)
}

// 获取服务端已存在的 Env 详细信息
pub(crate) fn get_env_info(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqGetEnvInfo,
    }

    // 汇聚各 Slave 的信息
    fn cb(r: &mut SlaveRes) {
        info_omit!(resp_cb_merge::<RespGetEnvInfo>(r));
    }

    let req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let addr_set = {
        let lk = ENV_MAP.read();
        let res = req
            .msg
            .env_set
            .iter()
            .filter_map(|env_id| lk.get(env_id))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        if res.is_empty() {
            let msg: HashMap<String, RespGetEnvInfo> = map! {};
            return send_ok!(req.uuid, msg, peeraddr);
        } else {
            res
        }
    };

    fwd_to_slave!(@@ops_id, req, peeraddr, cb, &addr_set)
}

/// 从已有 ENV 中踢出指定的 VM 实例
pub(crate) fn update_env_kick_vm(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvKickVm,
    }

    fwd_to_slave!(ops_id, request, MyReq, peeraddr)
}

/// 更新已有 Env 的生命周期
pub(crate) fn update_env_lifetime(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvLife,
    }

    let req = serde_json::from_slice::<MyReq>(&request).c(d!())?;

    if let Some(set) = ENV_MAP.read().get(&req.msg.env_id) {
        fwd_to_slave!(@@ops_id, req, peeraddr, resp_cb_simple, set)
    } else {
        send_err!(req.uuid, eg!("ENV not exists!"), peeraddr)
    }
}

/// 删除 Env
pub(crate) fn del_env(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqDelEnv,
    }

    let req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let addr_set = if let Some(set) = ENV_MAP.write().remove(&req.msg.env_id) {
        set
    } else {
        return send_ok!(req.uuid, "Success!", peeraddr);
    };

    fwd_to_slave!(@@ops_id, req, peeraddr, resp_cb_simple, &addr_set)
}

/// 获取服务端已存在的 Env 概略信息(全局)
#[inline(always)]
pub(crate) fn get_env_list_all(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    get_env_list(ops_id, peeraddr, request).c(d!())
}

/// 暂停运行, 让出资源
/// - 保留临时镜像和端口影射
/// - 停止所有 VM 进程
/// - 资源计数递减
pub(crate) fn stop_env(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqStopEnv,
    }

    fwd_to_slave!(ops_id, request, MyReq, peeraddr)
}

/// 恢复运行先前被 stop 的 ENV
/// - 启动所有 VM 进程
/// - 资源计数递增
pub(crate) fn start_env(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqStartEnv,
    }

    fwd_to_slave!(ops_id, request, MyReq, peeraddr)
}

/// 更新已有 ENV 中资源配置信息
pub(crate) fn update_env_resource(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    #[derive(Deserialize)]
    struct MyReq {
        uuid: u64,
        cli_id: CliId,
        msg: ReqUpdateEnvResource,
    }

    fwd_to_slave!(ops_id, request, MyReq, peeraddr)
}

/// 生成不重复的 uuid
fn gen_proxy_uuid() -> u64 {
    lazy_static! {
        static ref UUID: AtomicU64 = AtomicU64::new(9999);
    }
    UUID.fetch_add(1, Ordering::Relaxed)
}

// 用于回复 Client 的通用回调
fn resp_cb_simple(r: &mut SlaveRes) {
    if 0 < r.num_to_wait {
        send_err!(
            r.uuid,
            eg!("Not all slave-server[s] reponsed!"),
            r.peeraddr
        )
        .unwrap_or_else(|e| p(e));
    } else if r.msg.values().any(|v| v.status == RetStatus::Fail) {
        let msg = r
            .msg
            .values()
            .filter(|v| v.status == RetStatus::Fail)
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ;; ");
        send_err!(r.uuid, eg!(msg), r.peeraddr).unwrap_or_else(|e| p(e));
    } else {
        send_ok!(r.uuid, "Success!", r.peeraddr).unwrap_or_else(|e| p(e));
    }
}

// 汇聚各 Slave 的信息, 回复给 Client;
// 该回调仅用于查询类接口, 采用 “尽力而为” 模式, 部分返回即视为成功.
fn resp_cb_merge<'a, T: Serialize + Deserialize<'a>>(
    r: &'a mut SlaveRes,
) -> Result<()> {
    if 0 < r.num_to_wait {
        p(eg!("Not all slave-server[s] reponsed!"));
    }

    // if r.msg.values().any(|v| v.status == RetStatus::Fail) {
    //     p(eg!("Some slave-server[s] got error!"));
    // }

    let res = r
        .msg
        .iter()
        .filter(|(_, raw)| raw.status == RetStatus::Success)
        .filter_map(|(slave, raw)| {
            info!(serde_json::from_slice::<HashMap<ServerAddr, T>>(&raw.msg))
                .ok()
                .and_then(|resp| resp.into_iter().next())
                .map(|resp| (slave.to_string(), resp.1))
        })
        .collect::<HashMap<_, _>>();

    send_ok!(r.uuid, res, r.peeraddr)
}

/// 分发请求至各 Slave Server
fn send_req_to_slave<T: Serialize>(
    ops_id: usize,
    req: Req<T>,
    slave_set: &[SocketAddr],
) -> Result<()> {
    let mut req_bytes = serde_json::to_vec(&req)
        .c(d!())
        .and_then(|req| zlib::encode(&req[..]).c(d!()))?;
    let mut body =
        format!("{id:>0width$}", id = ops_id, width = OPS_ID_LEN).into_bytes();
    body.append(&mut req_bytes);

    macro_rules! do_send {
        ($body: expr, $slave: expr) => {
            task::spawn(async move {
                info_omit!(SOCK_MID.send_to(&$body, $slave).await);
            });
        };
    }

    if 1 < slave_set.len() {
        for slave in slave_set.iter().skip(1).copied() {
            let b = body.clone();
            do_send!(b, slave);
        }
    } else if slave_set.is_empty() {
        return Ok(());
    }

    // 非空的情况,
    // 单独处理第一个 slave,
    // 避免无谓的 clone
    let first_slave = slave_set[0];
    do_send!(body, first_slave);

    Ok(())
}

/// 将待回复的 handler 注册到 Proxy 中
fn register_resp_hdr(
    num_to_wait: usize,
    cb: fn(&mut SlaveRes),
    peeraddr: SockAddr,
    orig_uuid: UUID,
    proxy_uuid: UUID,
) {
    let ts = ts!();
    let idx = ts as usize % TIMEOUT_SECS;
    let sr = SlaveRes {
        msg: map! {},
        num_to_wait,
        start_ts: ts,
        do_resp: cb,
        peeraddr,
        uuid: orig_uuid,
    };
    let mut proxy = PROXY.lock();
    proxy.idx_map.insert(proxy_uuid, idx);

    // 清理已失效的 Bucket 内容,
    // 不能单纯依靠"清理线程"去处理,
    // 会存在错误地更新了本该过期的 Bucket 时间戳的现象,
    // 导致"清理线程"无法正确识别将被清理的对象.
    if ts != proxy.buckets[idx].ts {
        mem::take(&mut proxy.buckets[idx]).res.keys().for_each(|k| {
            proxy.idx_map.remove(k);
        });
    }

    // 而后才能更新时间戳
    proxy.buckets[idx].ts = ts;

    proxy.buckets[idx].res.insert(proxy_uuid, sr);
}
