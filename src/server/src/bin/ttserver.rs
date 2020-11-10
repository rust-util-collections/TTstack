use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use myutil::{err::*, *};
use std::{path::Path, process};
use ttserver::cfg::Cfg;

fn main() {
    pnk!(ttserver::start(pnk!(parse_cfg())));
}

/// 解析命令行参数
fn parse_cfg() -> Result<Cfg> {
    // 要添加 "--ignored" 等兼容 `cargo test` 的选项
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .args_from_usage("--serv-addr=[ADDR] '服务监听地址.'")
        .args_from_usage("--serv-port=[PORT] '服务监听端口.'")
        .args_from_usage("--log-path=[PATH] '日志存储路径.'")
        .args_from_usage("--image-path=[PATH] '镜像存放路径.'")
        .args_from_usage("--cfgdb-path=[PATH] 'Env Config 存放路径.'")
        .args_from_usage("--cpu-total=[NUM] '可以使用的 CPU 核心总数.'")
        .args_from_usage("--mem-total=[SIZE] '可以使用的内存总量, 单位: MB.'")
        .args_from_usage("--disk-total=[SIZE] '可以使用的磁盘总量, 单位: MB.'")
        .get_matches();

    match (
        matches.value_of("serv-addr"),
        matches.value_of("serv-port"),
        matches.value_of("log-path"),
        matches.value_of("image-path"),
        matches.value_of("cfgdb-path"),
        matches.value_of("cpu-total"),
        matches.value_of("mem-total"),
        matches.value_of("disk-total"),
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
            serv_ip: addr.to_owned(),
            serv_at: format!("{}:{}", addr, port.unwrap_or("9527")),
            log_path: log_path.map(|lp| lp.to_owned()),
            image_path: check_image_path(img_path).c(d!())?.to_owned(),
            cfgdb_path: cfgdb_path.to_owned(),
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
                matches.usage()
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
