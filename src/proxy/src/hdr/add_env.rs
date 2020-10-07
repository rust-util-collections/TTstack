//!
//! # add_env()
//!
//! 逻辑较复杂, 实现为一个单独的模块
//!

use super::*;
use std::collections::HashSet;

/// 创建新的 Env,
/// 不能简单的转发请求,
/// 要分割资源之后再分发
pub(super) fn add_env(
    ops_id: usize,
    peeraddr: SocketAddr,
    request: Vec<u8>,
) -> Result<()> {
    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    let slave_info = SLAVE_INFO.read().clone();
    let mut resource_pool = slave_info
        .clone()
        .into_iter()
        .map(|(k, v)| (k, (v, vct![])))
        .collect::<HashMap<_, _>>();

    let resource_ok = {
        let reqx = AddEnv::from(mem::take(&mut req.msg));
        let cpu_need = reqx.cpu_num * (1 + reqx.dup_each as u32);
        let mem_need = reqx.mem_size * (1 + reqx.dup_each as u32);
        let disk_need = reqx.disk_size * (1 + reqx.dup_each as u32);
        let res = slave_info
            .into_iter()
            .map(|(_, i)| i.supported_list.into_iter())
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|os| {
                reqx.os_prefix
                    .iter()
                    .any(|pre| os.starts_with(&pre.to_lowercase()))
            })
            .map(|os| {
                for r in resource_pool.values_mut() {
                    if r.0
                        .supported_list
                        .iter()
                        .any(|slave_os| slave_os == &os)
                        && r.0.cpu_total - r.0.cpu_used >= cpu_need
                        && r.0.mem_total - r.0.mem_used >= mem_need
                        && r.0.disk_total - r.0.disk_used >= disk_need
                    {
                        r.0.cpu_used += cpu_need;
                        r.0.mem_used += mem_need;
                        r.0.disk_used += disk_need;
                        r.1.push(os);
                        return String::new();
                    }
                }
                os
            })
            .all(|os| os.is_empty());

        // Convert back(MUST) !!!
        req.msg = ReqAddEnv::from(reqx);

        res
    };

    if !resource_ok {
        // 存在非空, 说明有不能满足的资源需求, 返回错误
        send_err!(req.uuid, eg!("Resource busy!"), peeraddr)
    } else {
        // os 全部被置为空, 说明所有的资源需求都可以满足, 向 Slave Server 分发请求
        let jobs = resource_pool
            .into_iter()
            .map(|(addr, (_, os_list))| (addr, os_list))
            .filter(|(_, os_list)| !os_list.is_empty())
            .collect::<Vec<_>>();

        send_req(ops_id, req, peeraddr, jobs).c(d!())
    }
}

fn send_req(
    ops_id: usize,
    mut req: MyReq,
    peeraddr: SocketAddr,
    jobs: Vec<(SocketAddr, Vec<String>)>,
) -> Result<()> {
    let num_to_wait = jobs.len();
    let proxy_uuid = gen_proxy_uuid();
    let cli_id = req
        .cli_id
        .take()
        .unwrap_or_else(|| peeraddr.ip().to_string());

    register_resp_hdr(
        num_to_wait,
        resp_cb_simple,
        peeraddr,
        req.uuid,
        proxy_uuid,
    );

    ENV_MAP.write().insert(
        req.msg.env_id.clone(),
        jobs.iter().map(|(sa, _)| sa).copied().collect(),
    );

    for (slave_addr, os_list) in jobs.into_iter() {
        let mut m = req.msg.clone();
        m.os_prefix = os_list;

        send_req_to_slave(
            ops_id,
            Req::newx(proxy_uuid, Some(cli_id.clone()), m),
            &[slave_addr],
        )
        .c(d!())
        .or_else(|e| send_err!(req.uuid, e, peeraddr))?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct MyReq {
    uuid: u64,
    cli_id: Option<CliId>,
    msg: ReqAddEnv,
}

struct AddEnv {
    pub env_id: EnvId,
    pub os_prefix: Vec<String>,
    pub life_time: Option<u64>,
    pub cpu_num: u32,
    pub mem_size: u32,
    pub disk_size: u32,
    pub port_set: Vec<Port>,
    pub dup_each: u32,
    pub deny_outgoing: bool,
}

impl From<ReqAddEnv> for AddEnv {
    fn from(req: ReqAddEnv) -> Self {
        AddEnv {
            env_id: req.env_id,
            os_prefix: req.os_prefix,
            life_time: req.life_time,
            cpu_num: req.cpu_num.unwrap_or(CPU_DEFAULT),
            mem_size: req.mem_size.unwrap_or(MEM_DEFAULT),
            disk_size: req.disk_size.unwrap_or(DISK_DEFAULT),
            port_set: req.port_set,
            dup_each: req.dup_each.unwrap_or(0),
            deny_outgoing: req.deny_outgoing,
        }
    }
}

impl From<AddEnv> for ReqAddEnv {
    fn from(x: AddEnv) -> Self {
        ReqAddEnv {
            env_id: x.env_id,
            os_prefix: x.os_prefix,
            life_time: x.life_time,
            cpu_num: Some(x.cpu_num),
            mem_size: Some(x.mem_size),
            disk_size: Some(x.disk_size),
            port_set: x.port_set,
            dup_each: Some(x.dup_each),
            deny_outgoing: x.deny_outgoing,
        }
    }
}
