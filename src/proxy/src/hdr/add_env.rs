//!
//! # add_env()
//!
//! 逻辑较复杂, 实现为一个单独的模块
//!

use super::*;
use std::{collections::HashSet, mem};

#[derive(Clone, Debug, Deserialize)]
struct MyReq {
    uuid: u64,
    cli_id: Option<CliId>,
    msg: ReqAddEnv,
}

/// 创建新的 Env,
/// 不能简单的转发请求,
/// 要分割资源之后再分发
pub(super) fn add_env(
    ops_id: usize,
    peeraddr: SockAddr,
    request: Vec<u8>,
) -> Result<()> {
    let mut req = serde_json::from_slice::<MyReq>(&request).c(d!())?;
    if ENV_MAP.read().get(&req.msg.env_id).is_some() {
        return Err(eg!("Aready exists!"));
    }

    let slave_info = SLAVE_INFO.read().clone();

    let mut resource_pool = slave_info
        .clone()
        .into_iter()
        .map(|(k, mut v)| {
            let supported_set = mem::take(&mut v.supported_list)
                .into_iter()
                .collect::<HashSet<_>>();
            (k, v, vct![], supported_set)
        })
        .collect::<Vec<_>>();

    req.msg.set_os_lowercase();
    let me = &req.msg;
    let dup_each = me.check_dup().c(d!())?;
    let rsc_wanted = slave_info
        .into_iter()
        .map(|(_, i)| i.supported_list.into_iter())
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .filter(|os| req.msg.os_prefix.iter().any(|pre| os.starts_with(pre)))
        .map(|os| {
            (0..(1 + dup_each)).map(move |_| VmCfgProxy {
                os: os.clone(),
                port_list: me.port_set.clone(),
                cpu_num: me.cpu_num,
                mem_size: me.mem_size,
                disk_size: me.disk_size,
                rand_uuid: me.rand_uuid,
            })
        })
        .flatten();

    let cpu_need = me.cpu_num.unwrap_or(CPU_DEFAULT);
    let mem_need = me.mem_size.unwrap_or(MEM_DEFAULT);
    let disk_need = me.disk_size.unwrap_or(DISK_DEFAULT);
    'x: for w in rsc_wanted {
        resource_pool.sort_by_key(|s| s.1.mem_used - s.1.mem_total);
        for i in resource_pool.iter_mut() {
            if i.3.contains(&w.os)
                && i.1.cpu_total - i.1.cpu_used >= cpu_need
                && i.1.mem_total - i.1.mem_used >= mem_need
                && i.1.disk_total - i.1.disk_used >= disk_need
            {
                i.1.cpu_used += cpu_need;
                i.1.mem_used += mem_need;
                i.1.disk_used += disk_need;
                i.2.push(w);
                continue 'x;
            }
        }

        return send_err!(
            req.uuid,
            eg!("Server has not enough resources to meet your needs!"),
            peeraddr
        );
    }

    // 任务分配完成, 执行分发
    let jobs = resource_pool
        .into_iter()
        .map(|(sa, _, v, _)| (sa, v))
        .filter(|(_, v)| !v.is_empty())
        .collect::<Vec<_>>();
    send_req(ops_id, req, peeraddr, jobs).c(d!())
}

fn send_req(
    ops_id: usize,
    mut req: MyReq,
    peeraddr: SockAddr,
    jobs: Vec<(SocketAddr, Vec<VmCfgProxy>)>,
) -> Result<()> {
    let num_to_wait = jobs.len();
    let proxy_uuid = gen_proxy_uuid();
    let cli_id = req.cli_id.take().unwrap_or_else(|| peeraddr.to_str());

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

    // 清理不需要的字段, 减少网络数据量
    req.msg.os_prefix = vct![];
    req.msg.cpu_num = None;
    req.msg.mem_size = None;
    req.msg.disk_size = None;
    req.msg.port_set = vct![];

    let mut m;
    for (slave_addr, vmcfg) in jobs.into_iter() {
        m = req.msg.clone();
        m.vmcfg = Some(vmcfg);

        send_req_to_slave(
            ops_id,
            Req::new(proxy_uuid, cli_id.clone(), m),
            &[slave_addr],
        )
        .c(d!())
        .or_else(|e| send_err!(req.uuid, e, peeraddr))?;
    }

    Ok(())
}
