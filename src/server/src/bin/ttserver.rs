use clap::{Arg, Command};
use ruc::*;
use std::{path::Path, process};
use ttserver::cfg::Cfg;

fn main() {
    pnk!(ttserver::start(pnk!(parse_cfg())));
}

/// 解析命令行参数
fn parse_cfg() -> Result<Cfg> {
    // 要添加 "--ignored" 等兼容 `cargo test` 的选项
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::new("serv-addr")
            .long("serv-addr")
            .value_name("ADDR")
            .help("服务监听地址."))
        .arg(Arg::new("serv-port")
            .long("serv-port")
            .value_name("PORT")
            .help("服务监听端口."))
        .arg(Arg::new("log-path")
            .long("log-path")
            .value_name("PATH")
            .help("日志存储路径."))
        .arg(Arg::new("image-path")
            .long("image-path")
            .value_name("PATH")
            .help("镜像存放路径."))
        .arg(Arg::new("cfgdb-path")
            .long("cfgdb-path")
            .value_name("PATH")
            .help("Env Config 存放路径."))
        .arg(Arg::new("cpu-total")
            .long("cpu-total")
            .value_name("NUM")
            .help("可以使用的 CPU 核心总数."))
        .arg(Arg::new("mem-total")
            .long("mem-total")
            .value_name("SIZE")
            .help("可以使用的内存总量, 单位: MB."))
        .arg(Arg::new("disk-total")
            .long("disk-total")
            .value_name("SIZE")
            .help("可以使用的磁盘总量, 单位: MB."))
        .get_matches();

    match (
        matches.get_one::<String>("serv-addr"),
        matches.get_one::<String>("serv-port"),
        matches.get_one::<String>("log-path"),
        matches.get_one::<String>("image-path"),
        matches.get_one::<String>("cfgdb-path"),
        matches.get_one::<String>("cpu-total"),
        matches.get_one::<String>("mem-total"),
        matches.get_one::<String>("disk-total"),
    ) {
        (
            Some(addr),
            port,
            log_path,
            Some(img_path),
            Some(cfgdb_path),
            Some(cpu),
            Some(mem),
            Some(disk),
        ) => Ok(Cfg {
            serv_ip: addr.clone(),
            serv_at: format!("{}:{}", addr, port.map(|s| s.as_str()).unwrap_or("9527")),
            log_path: log_path.map(|lp| lp.clone()),
            image_path: check_image_path(img_path).c(d!())?.to_owned(),
            cfgdb_path: cfgdb_path.clone(),
            cpu_total: cpu.parse::<i32>().c(d!())?,
            mem_total: mem.parse::<i32>().c(d!())?,
            disk_total: disk.parse::<i32>().c(d!())?,
        }),
        (addr, _, _, img_path, cfgdb_path, cpu, mem, disk) => {
            let msg = format!(
                "\x1b[01mOption missing: [{}]\x1b[00m",
                [
                    ("--serv-addr", addr),
                    ("--image-path", img_path),
                    ("--cfgdb-path", cfgdb_path),
                    ("--cpu-total", cpu),
                    ("--mem-total", mem),
                    ("--disk-total", disk)
                ]
                .iter()
                .filter(|(_, v)| v.is_none())
                .map(|(k, _)| k)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
            );

            eprintln!(
                "\n\x1b[31;01mInvalid arguments\x1b[00m\n\n{}\n\n{}\n",
                msg,
                matches.render_usage()
            );

            process::exit(1);
        }
    }
}

/// 确认镜像路径可写并且是目录
fn check_image_path(path: &str) -> Result<&str> {
    let p = Path::new(path);

    #[cfg(target_os = "Linux")]
    if p.metadata().c(d!())?.permissions().readonly() {
        return Err(eg!("无写权限!"));
    }

    if !p.is_dir() {
        return Err(eg!("不是目录!"));
    }

    Ok(path)
}
