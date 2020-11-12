//!
//! # Cfg File
//!

use lazy_static::lazy_static;
use myutil::{err::*, *};
use serde::{Deserialize, Serialize};
use std::{env, fs, net::SocketAddr, path};

lazy_static! {
    /// 客户端配置路径
    pub static ref CFG_PATH: String = format!("{}/.tt",pnk!(env::var("HOME")));
    /// 客户端配置文件
    pub static ref CFG_FILE: String = format!("{}/tt.json", CFG_PATH.as_str());
    /// 客户端配置信息
    pub static ref CFG: Cfg = pnk!(read_cfg());
}

/// 后续支持同时连接多个 server
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Cfg {
    /// 暂时只接受单个值
    pub server_list: Server,
    /// 本机别名
    pub client_id: String,
}

impl Cfg {
    /// TODO
    pub fn print_to_user(&self) {
        dbg!(self);
    }

    fn get_servaddr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.server_list.addr, self.server_list.port)
            .parse::<SocketAddr>()
            .c(d!())
    }
}

/// 转换平面地址为 SocketAddr
pub fn get_servaddr() -> Result<SocketAddr> {
    read_cfg().c(d!())?.get_servaddr().c(d!())
}

/// 服务端连接信息
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Server {
    /// IP 地址
    pub addr: String,
    /// 服务端口
    pub port: u16,
}

impl Server {
    /// 创建新的服务端连接实例
    pub fn new(addr: &str, port: u16) -> Server {
        Server {
            addr: addr.to_owned(),
            port,
        }
    }
}

/// 读取客户端配置文件
pub fn read_cfg() -> Result<Cfg> {
    fs::read(CFG_FILE.as_str())
        .c(d!())
        .and_then(|cfg| serde_json::from_slice(&cfg).c(d!()))
}

/// 写入客户端配置文件
pub fn write_cfg(cfg: &Cfg) -> Result<()> {
    serde_json::to_string_pretty(cfg)
        .c(d!())
        .and_then(|cfg| fs::write(CFG_FILE.as_str(), cfg).c(d!()))
}

/// 创建客户端配置路径和文件
pub fn cfg_init() -> Result<()> {
    if !path::Path::new(CFG_FILE.as_str()).exists() {
        fs::create_dir_all(CFG_PATH.as_str()).c(d!())?;
    }

    // 如果存在并且格式解析通过, 则什么都不做;
    // 否则就写入一个空文件
    read_cfg()
        .c(d!())
        .map(|_| ())
        .or_else(|_| write_cfg(&Cfg::default()).c(d!()))
        .and_then(|_| set_sshenv())
}

//////////////////////////////////////////////////////////////////

const RSA_PRIVATE: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA5Pwkml4qe8MGtsbA2mZunipshE/dXyNvLn3VTGb7Mpu7PMtE
FcXTterfSJWx/aIJ6f8yxEyUpdTMaBM9UdiPaxPkdHq2ngU/yyLBeZyMjAWf2ZOU
kTa6HIlvvcOAMVmR3LoBs51RSMW3gtiWv6uMcQ1kW7TweZ79kfdnk8GWek6L8Y5k
hl1HAVUVRYLxNJOyoHUQhivatCTlGTE9nd37PBZDiT3ks552sGlIi5S2BbGNUeBf
/tK/s5Ww4CqwZp479RmCqIQXBTLMNdJtPHFN1Iycpuu0dCNjSjABYVgWQIeCBjCH
4O8N1GtiuJx0MJXJzlrme485Zf52EZq0vU9yxQIDAQABAoIBAQDT/mPczoVCY0Ja
ARQWnnKW1+vzawUlyWZrgm/w9f5l0iu8kusLxUTFzRa+2mgYyuWmz28usT+Fb8d2
KynAFmBg39/HvrxG+9EdvaWlczvjfmmJQ8ptzl7rgIoFA3QxPB2AXmyo32KbnwDQ
kLiv5qB1IdLh3FguIPXdJ1GrR7SKsWDVooOEWJvJfbNxt/ak5gxow9LIN/2Wv1PM
xh/+C7Evx1auOkVuC9+2F7rJbb5M/B6B+YLqjOMtjIe1ME9VUqkSuf3bsYHpMVv5
LLVylEpfdtYG6fi1t9SA+P6e4DyH13zQLSN3QDjUdeeyIbXqe1FruiYpQtLo3A4r
6+rKaFjZAoGBAPurbsDvtAbxfR0YFKNov3nhE4QE71rr+B1VLGinnHX3I60tpY07
jhf+v1ntmCsXY64W8HFdn0rOpUXPhwsdyXi9lVBA9JFAICPyVu/o6mJXfJ8a1BMi
E96veImNDwuK/0Ae5VK1jcdouZgoG9co6AWmgueHx3D92fFs/Oirh+1nAoGBAOjs
yZpt1+gH1m3/skzUoKGgc+vk13LM42EtjTQy1mnWfks7Nl8lkDKrlyTqDJxeycTP
Cr7+bkqQkmQFHtzuBiSq7yt4KXOdlWYrsx9KSbYMXFPdugxfiOFQsidNAOLpobjh
QIJWmFYSQFWdVNv/YOQ8DCNbznt5VhlBF4UkY9bzAoGAd+862LdjE+wBs9vF+hnx
JiQdKM0xRCMwGsp8X2OBLLaaSe1299dp4AWHK1QPMHn1BwHnlB8JypywJpS/xoxr
dx7iCVzrME1fA8J5q9tT14nZ2fjvGC8lSPpWdzbB9L5I5kXTA5eB+YXu7JQwsFjO
OeMgfzY11aMkOem2nSshnAECgYBs1WcFz1lYw4C/+P+4wokjvDMt/8ljjLSZzYzy
3OYuodh1En+/SW/tHRwMVYf68JdabFtbDss97/tW3MWk+VrJe00xhH3p1bHfAYA6
mJ2EgJYLYcjyyxjMHsZ/co19eSjll+pqfEfFv9Vrq43hFZySSDRruRPrwbAnMLDq
tywnXQKBgG7MosZPAKKGUUpBqw6ZbkHy/K+qSUtKvyUVIXVBl8hQGeeKltcOVesG
UIkpfJfmyFlw52THX7jwc0RmHfIjjeit4vRW2Id2wUNRlgFW5OQu5PCncRhKpdJM
wUVL9Lu+n7huI+GtsmATdhUYlzE/K8nNmhJuw3abrGPPUgrtx5OE
-----END RSA PRIVATE KEY-----";

const SSH_SETTING: &str = "StrictHostKeyChecking no
UserKnownHostsFile /dev/null";

lazy_static! {
    static ref SSH_CFG_DIR: String =
        format!("{}/.ssh", pnk!(env::var("HOME")));
    static ref SSH_CFG_FILE: String =
        format!("{}/config", SSH_CFG_DIR.as_str());
    /// 连接 VM 所用的私钥在 client 上的路径
    pub static ref SSH_VM_KEY: String =
        format!("{}/tt_rsa", SSH_CFG_DIR.as_str());
}

/// 确保无交互的建立 SSH 连接
fn set_sshenv() -> Result<()> {
    fs::create_dir_all(SSH_CFG_DIR.as_str())
        .c(d!())
        .and_then(|_| fs::write(SSH_CFG_FILE.as_str(), SSH_SETTING).c(d!()))
        .and_then(|_| {
            omit!(fs::remove_file(SSH_VM_KEY.as_str()).c(d!()));
            fs::write(SSH_VM_KEY.as_str(), RSA_PRIVATE).c(d!())
        })
        .and_then(|_| {
            crate::cmd_exec("chmod", &["0400", SSH_VM_KEY.as_str()]).c(d!())
        })
}
