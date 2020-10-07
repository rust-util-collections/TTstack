use super::ipfw_exec;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use std::env;

// 初始化时一次性设置此环境变量,
// 同时, 只会被 register_serv_ip 使用一次,
// 即使后续环境异常变动也不会产生不良影响.
const VAR_SERV_IP: &str = "TT_SERV_IP";

// DNAT LOOKUP TABLE
pub(super) const DNAT_TABLE: &str = "tt_dnat";

// IPFW 初始化
#[inline(always)]
pub(in crate::freebsd) fn init(serv_ip: &str) -> Result<()> {
    env::set_var(VAR_SERV_IP, serv_ip);
    register_serv_ip();

    let arg = format!(
        "
        ipfw -qf nat flush || exit 1;
        sysctl net.inet.tcp.tso=0 || exit 1;

        ipfw table {0} destroy 2>/dev/null;
        ipfw table {0} create type flow:dst-ip,dst-port valtype nat || exit 1;

        ipfw -q add 10000 nat tablearg ip from any to me in flow 'table({0})' || exit 1;
        ipfw -q add 10001 nat global ip from 10.0.0.0/8 to not 10.0.0.0/8 out || exit 1;

        ipfw delete 10002 2>/dev/null;
        ipfw -q add 10002 allow ip from any to any || exit 1;
        ",
        DNAT_TABLE
    );

    ipfw_exec(&arg).c(d!())
}

// 一次性注册,
// 服务端 IP 地址
#[inline(always)]
pub(super) fn register_serv_ip() -> &'static str {
    lazy_static! {
        static ref IP: String = pnk!(env::var(VAR_SERV_IP));
    }
    &IP
}
