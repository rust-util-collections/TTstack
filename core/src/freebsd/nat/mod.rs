//!
//! # NAT
//!
//! IPFW in-kernel nat.
//!

mod env;

use crate::freebsd::vm::cmd_exec;
use crate::{Ipv4, PubPort, Vm, VmPort};
pub(in crate::freebsd) use env::init;
use env::DNAT_TABLE;
use myutil::{err::*, *};
use std::collections::HashMap;

// 添加新的规则集,
// nat_id = min([pub_port, ...]),
// 传入的参数是以 VM 为单位的, PubPort 区间是唯一的
pub(crate) fn set_rule(vm: &Vm) -> Result<()> {
    if let Some(id) = vm.port_map.values().min() {
        let nat_id = id;
        let serv_ip = serv_ip();

        let (kv_set, rdr_set): (Vec<String>, Vec<String>) = vm.port_map
            .iter()
            .map(|(vm_port, pub_port)| {
                (
                    format!("{},{} {}", serv_ip, pub_port, nat_id),
                    format!(
                        "redirect_port tcp {0}:{1} {2} redirect_port udp {0}:{1} {2}",
                        vm.ip,
                        vm_port,
                        pub_port,
                    )
                )
            }).unzip();

        let kv_set = kv_set.join(" ");
        let rdr_set = rdr_set.join(" ");

        let arg = format!(
            "
            ipfw table {} add {} || exit 1;
            ipfw -q nat {} config ip {} {} || exit 1;
            ",
            DNAT_TABLE, kv_set, nat_id, serv_ip, rdr_set,
        );

        ipfw_exec(&arg).c(d!())?;
    }

    Ok(())
}

// 清理指定端口对应的 NAT 规则,
// nat_id = min([pub_port, ...]),
// 传入的参数是以 VM 为单位的, PubPort 区间是唯一的
#[inline(always)]
pub(crate) fn clean_rule(vm_set: &[&Vm]) -> Result<()> {
    let port_set = vm_set
        .iter()
        .map(|vm| vm.port_map.values())
        .flatten()
        .collect::<Vec<_>>();

    if let Some(id) = port_set.iter().min() {
        let serv_ip = serv_ip();
        let k_set = port_set
            .iter()
            .map(|pub_port| format!("{},{}", serv_ip, pub_port))
            .collect::<Vec<_>>()
            .join(" ");

        let arg = format!(
            "
            ipfw -q nat {} delete;
            ipfw table {} delete {} || exit 1;
            ",
            id, DNAT_TABLE, k_set,
        );

        ipfw_exec(&arg).c(d!())?;
    }

    // TODO
    // allow_outgoing(vm_set).c(d!())

    Ok(())
}

// TODO
#[inline(always)]
pub(crate) fn deny_outgoing(_vm_set: &[&Vm]) -> Result<()> {
    Err(eg!("Unsupported feature!"))
}

// TODO
#[inline(always)]
pub(crate) fn allow_outgoing(_vm_set: &[&Vm]) -> Result<()> {
    Err(eg!("Unsupported feature!"))
}

// 执行 IPFW 命令
#[inline(always)]
fn ipfw_exec(arg: &str) -> Result<()> {
    cmd_exec("sh", &["-c", arg]).c(d!())
}

// 服务端 IP 地址
#[inline(always)]
fn serv_ip() -> &'static str {
    env::register_serv_ip()
}
