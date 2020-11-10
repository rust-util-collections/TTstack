//!
//! # Config Parse
//!

/// 配置信息
#[derive(Debug)]
pub struct Cfg {
    /// 日志存储路径
    pub log_path: Option<String>,
    /// eg: '10.10.10.22'
    pub serv_ip: String,
    /// 服务地址和端口,
    /// eg: '10.10.10.22:9527'
    pub serv_at: String,
    /// 基础镜像的存放路径,
    /// 同时也是服务进程的工作路径;
    /// 需要可写权限,
    /// tap.sh 会创建在此路径下,
    /// Vm 镜像也会创建在相同的跟径下
    pub image_path: String,
    /// Env Config 存放路径
    pub cfgdb_path: String,
    /// CPU 核心总数
    pub cpu_total: i32,
    /// Mem 总容量, 单位: MB
    pub mem_total: i32,
    /// Disk 总容量, 单位: MB
    pub disk_total: i32,
}

pub(crate) fn register_cfg(cfg: Option<Cfg>) -> Option<&'static Cfg> {
    static mut CFG: Option<Cfg> = None;
    if cfg.is_some() {
        unsafe {
            CFG = cfg;
        }
    }
    unsafe { CFG.as_ref() }
}
