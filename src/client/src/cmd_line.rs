//!
//! # Cmd Line
//!

use crate::ops::{config::*, env::*, status::*};
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, Arg,
    ArgMatches, SubCommand,
};
use myutil::{err::*, *};
use std::{path::Path, process};

// 报错并退出
macro_rules! err {
    ($app: expr) => {
        err!("", $app);
    };
    ($msg: expr, $app: expr) => {{
        eprintln!(
            "\n\x1b[31;01mInvalid arguments\x1b[00m\t{}\n\n{}\n",
            $msg,
            $app.usage()
        );
        process::exit(1);
    }};
}

macro_rules! option_num_parse {
    ($var: tt, $default: expr, $ty: ty) => {{
        if let Some($var) = $var {
            $var.parse::<$ty>().c(d!(not_num!($var)))?
        } else {
            $default
        }
    }};
}

/// errmsg for 'NOT a number'
macro_rules! not_num {
    ($num: expr) => {
        format!("{}({}) is NOT be a number!", stringify!($num), $num)
    };
}

/// 解析命令行参数
pub fn parse_and_exec() -> Result<()> {
    let m = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommands(vct![
            SubCommand::with_name("config")
            .arg_from_usage("-a, --server-addr=[ADDR] '服务端的监听地址'")
            .arg_from_usage("-p, --server-port=[PORT] '服务端的监听端口'")
            .arg_from_usage("-n, --client-id=[NAME] '客户端别名'"),
            SubCommand::with_name("status")
            .arg_from_usage("-c, --client '查看客户端状态'")
            .arg_from_usage("-s, --server '查看服务端状态'"),
            SubCommand::with_name("env").subcommands(vct![
                SubCommand::with_name("add")
                    .arg(Arg::with_name("ENV").required(true).help("待创建的环境名称."))
                    .arg_from_usage("-n, --deny-outgoing '禁止虚拟机对外连网'")
                    .arg_from_usage("-l, --life-time=[TIME] '虚拟机的生命周期, 单位: 秒'")
                    .arg_from_usage("-C, --cpu-num=[CPU_SIZE] '虚拟机的 CPU 核心数量'")
                    .arg_from_usage("-M, --mem-size=[MEM_SIZE] '虚拟机的内存容量, 单位: MB'")
                    .arg_from_usage("-D, --disk-size=[DISK_SIZE] '虚拟机的磁盘容量, 单位: MB'")
                    .arg_from_usage("-d, --dup-each=[NUM] '每种虚拟机类型启动的实例数量'")
                    .arg_from_usage("-s, --os-prefix=[OS]... '虚拟机的系统, 如: CentOS7.x 等'")
                    .arg_from_usage("-p, --vm-port=[PORT]... '虚拟机需要开放的网络端口'")
                    .arg_from_usage("--same-uuid '所有虚拟机都使用同一个 UUID'"),
                SubCommand::with_name("del")
                .arg(
                    Arg::with_name("ENV")
                        .required(true)
                        .multiple(true)
                        .help("一个或多个环境名称.")
                ),
                SubCommand::with_name("stop")
                .arg(
                    Arg::with_name("ENV")
                        .required(true)
                        .multiple(true)
                        .help("一个或多个环境名称.")
                ),
                SubCommand::with_name("start")
                .arg(
                    Arg::with_name("ENV")
                        .required(true)
                        .multiple(true)
                        .help("一个或多个环境名称.")
                ),
                SubCommand::with_name("list"),
                SubCommand::with_name("listall"),
                SubCommand::with_name("show")
                .arg(
                    Arg::with_name("ENV")
                        .required(true)
                        .multiple(true)
                        .help("一个或多个环境名称.")
                ),
                SubCommand::with_name("update")
                    .arg(
                        Arg::with_name("ENV")
                            .required(true)
                            .multiple(true)
                            .help("一个或多个环境名称.")
                    )
                    .arg_from_usage("--SSSS '指定任意生命周期'")
                    .arg_from_usage("-l, --life-time=[TIME] '新的生命周期'")
                    .arg_from_usage("-C, --cpu-num=[CPU_SIZE] '新的 CPU 数量'")
                    .arg_from_usage("-M, --mem-size=[MEM_SIZE] '新的内存容量, 单位: MB'")
                    .arg_from_usage("-p, --vm-port=[PORT]... '新的网络端口集合(全量替换, 非增量计算)'")
                    .arg_from_usage("-n, --deny-outgoing '禁止虚拟机对外连网'")
                    .arg_from_usage("-y, --allow-outgoing '允许虚拟机对外连网'")
                    .args_from_usage("--kick-dead '清除所有失去响应的 VM 实例'")
                    .args_from_usage("--kick-vm=[VM_ID]... '待剔除的 VM 的 ID'")
                    .args_from_usage("--kick-os=[OS_PREFIX]... '待剔除的系统名称前缀'"),
                SubCommand::with_name("get")
                    .arg_from_usage("--use-ssh '使用 SSH 协议通信'")
                    .arg_from_usage("-f, --file-path=[PATH] '文件在远程的路径'")
                    .arg_from_usage("-t, --time-out=[TIME] '可执行的最长时间, 单位: 秒'")
                    .arg_from_usage("-s, --os-prefix=[OS]... '按系统名称前缀筛选'")
                    .arg_from_usage("-m, --vm-id=[VM]... '按 VmId 精确筛选'")
                    .arg(
                        Arg::with_name("ENV")
                            .required(true)
                            .help("一个或多个环境名称.")
                            .multiple(true)
                    ),
                SubCommand::with_name("push")
                    .arg_from_usage("--use-ssh '使用 SSH 协议通信'")
                    .arg_from_usage("-f, --file-path=[PATH] '文件在本地的路径'")
                    .arg_from_usage("-t, --time-out=[TIME] '可执行的最长时间, 单位: 秒'")
                    .arg_from_usage("-s, --os-prefix=[OS]... '按系统名称前缀筛选'")
                    .arg_from_usage("-m, --vm-id=[VM]... '按 VmId 精确筛选'")
                    .arg(
                        Arg::with_name("ENV")
                            .required(true)
                            .help("一个或多个环境名称.")
                            .multiple(true)
                    ),
                SubCommand::with_name("run")
                    .arg_from_usage("--use-ssh '使用 SSH 协议通信'")
                    .arg_from_usage("-c, --cmd=[CMD] 'SHELL 命令'")
                    .arg_from_usage("-i, --interactive '交互式串行操作'")
                    .arg_from_usage("-x, --config-hawk '注册到 HAWK 监控系统'")
                    .arg_from_usage("-f, --script=[PATH] '脚本文件的本地路径'")
                    .arg_from_usage("-t, --time-out=[TIME] '可执行的最长时间, 单位: 秒'")
                    .arg_from_usage("-s, --os-prefix=[OS]... '按系统名称前缀筛选'")
                    .arg_from_usage("-m, --vm-id=[VM]... '按 VmId 精确筛选'")
                    .arg(
                        Arg::with_name("ENV")
                            .required(true)
                            .help("一个或多个环境名称.")
                            .multiple(true)
                    ),
            ])
        ])
        .get_matches();

    macro_rules! deal {
        ($hdr: tt, $m: expr) => {
            $hdr($m).unwrap_or_else(|e| err!(e, $m)).do_req().c(d!())
        };
    }

    match m.subcommand() {
        ("env", Some(env_m)) => match env_m.subcommand() {
            ("add", Some(add_m)) => deal!(env_add, add_m),
            ("del", Some(del_m)) => deal!(env_del, del_m),
            ("stop", Some(stop_m)) => deal!(env_stop, stop_m),
            ("start", Some(start_m)) => deal!(env_start, start_m),
            ("list", _) => EnvList.do_req().c(d!()),
            ("listall", _) => EnvListAll.do_req().c(d!()),
            ("show", Some(show_m)) => deal!(env_show, show_m),
            ("update", Some(update_m)) => deal!(env_update, update_m),
            ("get", Some(get_m)) => deal!(env_get, get_m),
            ("push", Some(push_m)) => deal!(env_push, push_m),
            ("run", Some(run_m)) => deal!(env_run, run_m),
            (_, _) => err!(env_m),
        },
        ("status", Some(status_m)) => {
            let mut req = Status::default();
            if status_m.is_present("server") {
                req.server = true;
            }
            req.do_req().c(d!())
        }
        ("config", Some(config_m)) => deal!(config, config_m),
        _ => err!(m),
    }
}

/// 解析 `tt env add ...`
fn env_add<'a>(m: &'a ArgMatches<'a>) -> Result<EnvAdd<'a>> {
    match (
        m.value_of("ENV"),
        m.values_of("os-prefix"),
        m.values_of("vm-port"),
        m.value_of("life-time"),
        m.value_of("cpu-num"),
        m.value_of("mem-size"),
        m.value_of("disk-size"),
        m.value_of("dup-each"),
    ) {
        (
            Some(env_id),
            Some(os_prefix),
            vm_port,
            life_time,
            cpu_num,
            mem_size,
            disk_size,
            dup_each,
        ) => Ok(EnvAdd {
            env_id,
            os_prefix: os_prefix.collect(),
            vm_port: {
                let mut port_set = vct![];
                if let Some(vm_port) = vm_port {
                    for p in vm_port {
                        if let Ok(port) = p.parse::<u16>() {
                            port_set.push(port);
                        } else {
                            return Err(eg!(format!("Invalid port: {}", p)));
                        }
                    }
                }
                port_set
            },
            life_time: option_num_parse!(life_time, 0, u64),
            cpu_num: option_num_parse!(cpu_num, 0, u8),
            mem_size: option_num_parse!(mem_size, 0, u16),
            disk_size: option_num_parse!(disk_size, 0, u32),
            dup_each: option_num_parse!(dup_each, 0, u16),
            deny_outgoing: m.is_present("deny-outgoing"),
            rand_uuid: !m.is_present("same-uuid"),
        }),
        _ => Err(eg!()),
    }
}

/// 解析 `tt env del ...`
fn env_del<'a>(m: &'a ArgMatches<'a>) -> Result<EnvDel<'a>> {
    if let Some(env_set) = m.values_of("ENV") {
        Ok(EnvDel {
            env_set: env_set.collect(),
        })
    } else {
        Err(eg!("没有指定 ENV"))
    }
}

/// 解析 `tt env stop ...`
fn env_stop<'a>(m: &'a ArgMatches<'a>) -> Result<EnvStop<'a>> {
    if let Some(env_set) = m.values_of("ENV") {
        Ok(EnvStop {
            env_set: env_set.collect(),
        })
    } else {
        Err(eg!("没有指定 ENV"))
    }
}

/// 解析 `tt env start ...`
fn env_start<'a>(m: &'a ArgMatches<'a>) -> Result<EnvStart<'a>> {
    if let Some(env_set) = m.values_of("ENV") {
        Ok(EnvStart {
            env_set: env_set.collect(),
        })
    } else {
        Err(eg!("没有指定 ENV"))
    }
}

/// 解析 `tt env show ...`
fn env_show<'a>(m: &'a ArgMatches<'a>) -> Result<EnvShow<'a>> {
    if let Some(env_set) = m.values_of("ENV") {
        Ok(EnvShow {
            env_set: env_set.collect(),
        })
    } else {
        Err(eg!("没有指定 ENV"))
    }
}

/// 解析 `tt env update ...`
fn env_update<'a>(m: &'a ArgMatches<'a>) -> Result<EnvUpdate<'a>> {
    match (
        m.value_of("life-time"),
        m.values_of("kick-vm"),
        m.values_of("kick-os"),
        m.value_of("cpu-num"),
        m.value_of("mem-size"),
        m.values_of("vm-port"),
        m.values_of("ENV"),
    ) {
        (_, _, _, _, _, _, None) => Err(eg!("没有指定 ENV")),
        (
            life_time,
            vm_id,
            os_prefix,
            cpu_num,
            mem_size,
            vm_port,
            Some(env_set),
        ) => Ok(EnvUpdate {
            env_set: env_set.collect(),
            kick_dead: m.is_present("kick-dead"),
            vm_id: if let Some(i) = vm_id {
                let mut res = vct![];
                for id in i {
                    res.push(id.parse::<i32>().c(d!(not_num!(id)))?);
                }
                res
            } else {
                vct![]
            },
            os_prefix: os_prefix
                .map(|os| os.map(|o| o.to_owned()).collect())
                .unwrap_or_default(),
            cpu_num: if let Some(n) = cpu_num {
                Some(n.parse::<u8>().c(d!(not_num!(n)))?)
            } else {
                None
            },
            mem_size: if let Some(n) = mem_size {
                Some(n.parse::<u16>().c(d!(not_num!(n)))?)
            } else {
                None
            },
            vm_port: if let Some(p) = vm_port {
                let mut res = vct![];
                for port in p {
                    res.push(port.parse::<u16>().c(d!(not_num!(port)))?);
                }
                res
            } else {
                vct![]
            },
            life_time: if let Some(lt) = life_time {
                Some(lt.parse::<u64>().c(d!(not_num!(lt)))?)
            } else {
                None
            },
            is_fucker: m.is_present("SSSS"),
            deny_outgoing: if m.is_present("deny-outgoing") {
                Some(true)
            } else if m.is_present("allow-outgoing") {
                Some(false)
            } else {
                None
            },
        }),
    }
}

/// 解析 `tt env get ...`
fn env_get<'a>(m: &'a ArgMatches<'a>) -> Result<EnvGet<'a>> {
    // 默认 10s 超时
    const TIMEOUT: &str = "10";

    match (
        m.value_of("file-path"),
        m.value_of("time-out"),
        m.values_of("os-prefix"),
        m.values_of("vm-id"),
        m.values_of("ENV"),
    ) {
        (None, _, _, _, _) => Err(eg!("没有指定 --file-path")),
        (_, _, _, _, None) => Err(eg!("没有指定 ENV")),
        (Some(file_path), timeout, osset, idset, Some(env_set)) => {
            Ok(EnvGet {
                use_ssh: m.is_present("use-ssh"),
                file_path,
                time_out: timeout.unwrap_or(TIMEOUT).parse::<u64>().c(d!())?,
                env_set: env_set.collect(),
                filter_os_prefix: osset
                    .map(|os| os.collect())
                    .unwrap_or_default(),
                filter_vm_id: if let Some(idset) = idset {
                    let mut res = vct![];
                    for id in idset.into_iter() {
                        res.push(id.parse::<i32>().c(d!(not_num!(id)))?);
                    }
                    res
                } else {
                    vct![]
                },
            })
        }
    }
}

/// 解析 `tt env push ...`
fn env_push<'a>(m: &'a ArgMatches<'a>) -> Result<EnvPush<'a>> {
    // 默认 10s 超时
    const TIMEOUT: &str = "10";

    match (
        m.value_of("file-path"),
        m.value_of("time-out"),
        m.values_of("os-prefix"),
        m.values_of("vm-id"),
        m.values_of("ENV"),
    ) {
        (None, _, _, _, _) => Err(eg!("没有指定 --file-path")),
        (_, _, _, _, None) => Err(eg!("没有指定 ENV")),
        (Some(file_path), timeout, osset, idset, Some(env_set)) => {
            Ok(EnvPush {
                use_ssh: m.is_present("use-ssh"),
                file_path: check_file_path(file_path).c(d!())?,
                time_out: timeout.unwrap_or(TIMEOUT).parse::<u64>().c(d!())?,
                env_set: env_set.collect(),
                filter_os_prefix: osset
                    .map(|os| os.collect())
                    .unwrap_or_default(),
                filter_vm_id: if let Some(idset) = idset {
                    let mut res = vct![];
                    for id in idset.into_iter() {
                        res.push(id.parse::<i32>().c(d!(not_num!(id)))?);
                    }
                    res
                } else {
                    vct![]
                },
            })
        }
    }
}

/// 解析 `tt env run ...`
fn env_run<'a>(m: &'a ArgMatches<'a>) -> Result<EnvRun<'a>> {
    // 默认 3s 超时
    const TIMEOUT: &str = "3";

    if let Some(env_set) = m.values_of("ENV") {
        let filter_os_prefix = m
            .values_of("os-prefix")
            .map(|os| os.collect())
            .unwrap_or_default();
        let filter_vm_id = if let Some(idset) = m.values_of("vm-id") {
            let mut res = vct![];
            for id in idset.into_iter() {
                res.push(id.parse::<i32>().c(d!(not_num!(id)))?);
            }
            res
        } else {
            vct![]
        };
        let to = m
            .value_of("time-out")
            .unwrap_or(TIMEOUT)
            .parse::<u64>()
            .c(d!())?;

        let env_set = env_set.collect();
        if m.is_present("config-hawk") {
            Ok(EnvRun {
                cmd: "",
                script: "",
                time_out: to,
                env_set,
                use_ssh: m.is_present("use-ssh"),
                interactive: false,
                config_hawk: true,
                filter_vm_id,
                filter_os_prefix,
            })
        } else if m.is_present("interactive") {
            Ok(EnvRun {
                cmd: "",
                script: "",
                time_out: to,
                env_set,
                use_ssh: true,
                interactive: true,
                config_hawk: false,
                filter_vm_id,
                filter_os_prefix,
            })
        } else {
            match (m.value_of("cmd"), m.value_of("script")) {
                (None, None) => Err(eg!("没有指定 --cmd 或 --script")),
                (cmd, Some(script)) => Ok(EnvRun {
                    cmd: cmd.unwrap_or(""),
                    script: check_file_path(script).c(d!())?,
                    time_out: to,
                    env_set,
                    use_ssh: m.is_present("use-ssh"),
                    interactive: false,
                    config_hawk: false,
                    filter_vm_id,
                    filter_os_prefix,
                }),
                (Some(cmd), None) => Ok(EnvRun {
                    cmd,
                    script: "",
                    time_out: to,
                    env_set,
                    use_ssh: m.is_present("use-ssh"),
                    interactive: false,
                    config_hawk: false,
                    filter_vm_id,
                    filter_os_prefix,
                }),
            }
        }
    } else {
        Err(eg!("没有指定 ENV"))
    }
}

/// 解析 `tt config ...`
fn config<'a>(m: &'a ArgMatches<'a>) -> Result<Config<'a>> {
    match (
        m.value_of("server-addr"),
        m.value_of("server-port"),
        m.value_of("client-id"),
    ) {
        (None, _, None) => Err(eg!("没有指定 --server-addr 或 --client-id")),
        (Some(server_addr), server_port, client_id) => Ok(Config {
            server_addr,
            server_port: option_num_parse!(server_port, 9527, u16),
            client_id: client_id.unwrap_or(""),
        }),
        (server_addr, server_port, Some(client_id)) => Ok(Config {
            server_addr: server_addr.unwrap_or(""),
            server_port: option_num_parse!(server_port, 9527, u16),
            client_id,
        }),
    }
}

/// 确认目标的类型是文件
#[inline(always)]
fn check_file_path(path: &str) -> Result<&str> {
    if Path::new(path).is_file() {
        Ok(path)
    } else {
        Err(eg!(format!("{} is NOT a file!", path)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn t_check_file_path() {
        assert!(check_file_path("/tmp").is_err());
        assert!(check_file_path("/etc/fstab").is_ok());
        assert_eq!(pnk!(check_file_path("/etc/fstab")), "/etc/fstab");
    }
}
