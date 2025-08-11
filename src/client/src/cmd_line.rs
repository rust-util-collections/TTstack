//!
//! # Cmd Line
//!

use crate::ops::{config::*, env::*, status::*};
use clap::{Arg, ArgMatches, Command};
use ruc::*;
use std::{path::Path, process};

// Report error and exit
macro_rules! err {
    ($app: expr) => {
        err!("", $app);
    };
    ($msg: expr, $app: expr) => {{
        eprintln!(
            "\n\x1b[31;01mInvalid arguments\x1b[00m\t{}\n\n{}\n",
            $msg,
            $app.render_usage()
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

/// Parse command line arguments
pub fn parse_and_exec() -> Result<()> {
    let m = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommands(vct![
            Command::new("config")
            .arg(Arg::new("server-addr")
                .short('a')
                .long("server-addr")
                .value_name("ADDR")
                .help("Server listening address"))
            .arg(Arg::new("server-port")
                .short('p')
                .long("server-port")
                .value_name("PORT")
                .help("Server listening port"))
            .arg(Arg::new("client-id")
                .short('n')
                .long("client-id")
                .value_name("NAME")
                .help("Client alias")),
            Command::new("status")
            .arg(Arg::new("client")
                .short('c')
                .long("client")
                .action(clap::ArgAction::SetTrue)
                .help("View client status"))
            .arg(Arg::new("server")
                .short('s')
                .long("server")
                .action(clap::ArgAction::SetTrue)
                .help("View server status")),
            Command::new("env").subcommands(vct![
                Command::new("add")
                    .arg(Arg::new("ENV")
                        .required(true)
                        .help("Name of environment to be created"))
                    .arg(Arg::new("deny-outgoing")
                        .short('n')
                        .long("deny-outgoing")
                        .action(clap::ArgAction::SetTrue)
                        .help("Prohibit virtual machine from connecting to external network"))
                    .arg(Arg::new("life-time")
                        .short('l')
                        .long("life-time")
                        .value_name("TIME")
                        .help("Virtual machine lifecycle, unit: seconds"))
                    .arg(Arg::new("cpu-num")
                        .short('C')
                        .long("cpu-num")
                        .value_name("CPU_SIZE")
                        .help("Number of CPU cores for virtual machine"))
                    .arg(Arg::new("mem-size")
                        .short('M')
                        .long("mem-size")
                        .value_name("MEM_SIZE")
                        .help("Virtual machine memory capacity, unit: MB"))
                    .arg(Arg::new("disk-size")
                        .short('D')
                        .long("disk-size")
                        .value_name("DISK_SIZE")
                        .help("Virtual machine disk capacity, unit: MB"))
                    .arg(Arg::new("dup-each")
                        .short('d')
                        .long("dup-each")
                        .value_name("NUM")
                        .help("Number of instances to start for each virtual machine type"))
                    .arg(Arg::new("os-prefix")
                        .short('s')
                        .long("os-prefix")
                        .value_name("OS")
                        .action(clap::ArgAction::Append)
                        .help("Virtual machine system, e.g: CentOS7.x etc"))
                    .arg(Arg::new("vm-port")
                        .short('p')
                        .long("vm-port")
                        .value_name("PORT")
                        .action(clap::ArgAction::Append)
                        .help("Network ports that virtual machine needs to open"))
                    .arg(Arg::new("same-uuid")
                        .long("same-uuid")
                        .action(clap::ArgAction::SetTrue)
                        .help("All virtual machines use the same UUID")),
                Command::new("del")
                .arg(Arg::new("ENV")
                        .required(true)
                        .action(clap::ArgAction::Append)
                        .help("One or more environment names")
                ),
                Command::new("stop")
                .arg(Arg::new("ENV")
                        .required(true)
                        .action(clap::ArgAction::Append)
                        .help("One or more environment names")
                ),
                Command::new("start")
                .arg(Arg::new("ENV")
                        .required(true)
                        .action(clap::ArgAction::Append)
                        .help("One or more environment names")
                ),
                Command::new("list"),
                Command::new("listall"),
                Command::new("show")
                .arg(Arg::new("ENV")
                        .required(true)
                        .action(clap::ArgAction::Append)
                        .help("One or more environment names")
                ),
                Command::new("update")
                    .arg(Arg::new("ENV")
                            .required(true)
                            .action(clap::ArgAction::Append)
                            .help("One or more environment names")
                    )
                    .arg(Arg::new("SSSS")
                        .long("SSSS")
                        .action(clap::ArgAction::SetTrue)
                        .help("Specify arbitrary lifecycle"))
                    .arg(Arg::new("life-time")
                        .short('l')
                        .long("life-time")
                        .value_name("TIME")
                        .help("New lifecycle"))
                    .arg(Arg::new("cpu-num")
                        .short('C')
                        .long("cpu-num")
                        .value_name("CPU_SIZE")
                        .help("New CPU count"))
                    .arg(Arg::new("mem-size")
                        .short('M')
                        .long("mem-size")
                        .value_name("MEM_SIZE")
                        .help("New memory capacity, unit: MB"))
                    .arg(Arg::new("vm-port")
                        .short('p')
                        .long("vm-port")
                        .value_name("PORT")
                        .action(clap::ArgAction::Append)
                        .help("New network port set (full replacement, not incremental)"))
                    .arg(Arg::new("deny-outgoing")
                        .short('n')
                        .long("deny-outgoing")
                        .action(clap::ArgAction::SetTrue)
                        .help("Prohibit virtual machine from connecting to external network"))
                    .arg(Arg::new("allow-outgoing")
                        .short('y')
                        .long("allow-outgoing")
                        .action(clap::ArgAction::SetTrue)
                        .help("Allow virtual machine to connect to external network"))
                    .arg(Arg::new("kick-dead")
                        .long("kick-dead")
                        .action(clap::ArgAction::SetTrue)
                        .help("Remove all unresponsive VM instances"))
                    .arg(Arg::new("kick-vm")
                        .long("kick-vm")
                        .value_name("VM_ID")
                        .action(clap::ArgAction::Append)
                        .help("ID of VM to be kicked"))
                    .arg(Arg::new("kick-os")
                        .long("kick-os")
                        .value_name("OS_PREFIX")
                        .action(clap::ArgAction::Append)
                        .help("OS prefix to be removed")),
                Command::new("get")
                    .arg(Arg::new("use-ssh")
                        .long("use-ssh")
                        .action(clap::ArgAction::SetTrue)
                        .help("Use SSH protocol for communication"))
                    .arg(Arg::new("file-path")
                        .short('f')
                        .long("file-path")
                        .value_name("PATH")
                        .help("File path on remote system"))
                    .arg(Arg::new("time-out")
                        .short('t')
                        .long("time-out")
                        .value_name("TIME")
                        .help("Maximum execution time in seconds"))
                    .arg(Arg::new("os-prefix")
                        .short('s')
                        .long("os-prefix")
                        .value_name("OS")
                        .action(clap::ArgAction::Append)
                        .help("Filter by OS name prefix"))
                    .arg(Arg::new("vm-id")
                        .short('m')
                        .long("vm-id")
                        .value_name("VM")
                        .action(clap::ArgAction::Append)
                        .help("Filter by exact VM ID"))
                    .arg(Arg::new("ENV")
                            .required(true)
                            .help("One or more environment names")
                            .action(clap::ArgAction::Append)
                    ),
                Command::new("push")
                    .arg(Arg::new("use-ssh")
                        .long("use-ssh")
                        .action(clap::ArgAction::SetTrue)
                        .help("Use SSH protocol for communication"))
                    .arg(Arg::new("file-path")
                        .short('f')
                        .long("file-path")
                        .value_name("PATH")
                        .help("Local file path"))
                    .arg(Arg::new("time-out")
                        .short('t')
                        .long("time-out")
                        .value_name("TIME")
                        .help("Maximum execution time in seconds"))
                    .arg(Arg::new("os-prefix")
                        .short('s')
                        .long("os-prefix")
                        .value_name("OS")
                        .action(clap::ArgAction::Append)
                        .help("Filter by OS name prefix"))
                    .arg(Arg::new("vm-id")
                        .short('m')
                        .long("vm-id")
                        .value_name("VM")
                        .action(clap::ArgAction::Append)
                        .help("Filter by exact VM ID"))
                    .arg(Arg::new("ENV")
                            .required(true)
                            .help("One or more environment names")
                            .action(clap::ArgAction::Append)
                    ),
                Command::new("run")
                    .arg(Arg::new("use-ssh")
                        .long("use-ssh")
                        .action(clap::ArgAction::SetTrue)
                        .help("Use SSH protocol for communication"))
                    .arg(Arg::new("cmd")
                        .short('c')
                        .long("cmd")
                        .value_name("CMD")
                        .help("Shell command"))
                    .arg(Arg::new("interactive")
                        .short('i')
                        .long("interactive")
                        .action(clap::ArgAction::SetTrue)
                        .help("Interactive serial operation"))
                    .arg(Arg::new("config-hawk")
                        .short('x')
                        .long("config-hawk")
                        .action(clap::ArgAction::SetTrue)
                        .help("Register to HAWK monitoring system"))
                    .arg(Arg::new("script")
                        .short('f')
                        .long("script")
                        .value_name("PATH")
                        .help("Local path to script file"))
                    .arg(Arg::new("time-out")
                        .short('t')
                        .long("time-out")
                        .value_name("TIME")
                        .help("Maximum execution time in seconds"))
                    .arg(Arg::new("os-prefix")
                        .short('s')
                        .long("os-prefix")
                        .value_name("OS")
                        .action(clap::ArgAction::Append)
                        .help("Filter by OS name prefix"))
                    .arg(Arg::new("vm-id")
                        .short('m')
                        .long("vm-id")
                        .value_name("VM")
                        .action(clap::ArgAction::Append)
                        .help("Filter by exact VM ID"))
                    .arg(Arg::new("ENV")
                            .required(true)
                            .help("One or more environment names")
                            .action(clap::ArgAction::Append)
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

/// Parse `tt env add ...`
fn env_add<'a>(m: &'a ArgMatches<'a>) -> Result<EnvAdd<'a>> {
    match (
        m.get_one::<String>("ENV"),
        m.get_many::<String>("os-prefix"),
        m.get_many::<String>("vm-port"),
        m.get_one::<String>("life-time"),
        m.get_one::<String>("cpu-num"),
        m.get_one::<String>("mem-size"),
        m.get_one::<String>("disk-size"),
        m.get_one::<String>("dup-each"),
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
            os_prefix: os_prefix.map(|v| v.cloned().collect()).unwrap_or_default(),
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
            deny_outgoing: m.get_flag("deny-outgoing"),
            rand_uuid: !m.get_flag("same-uuid"),
        }),
        _ => Err(eg!()),
    }
}

/// Parse `tt env del ...`
fn env_del<'a>(m: &'a ArgMatches<'a>) -> Result<EnvDel<'a>> {
    if let Some(env_set) = m.get_many::<String>("ENV") {
        Ok(EnvDel {
            env_set: env_set.cloned().collect(),
        })
    } else {
        Err(eg!("ENV not specified"))
    }
}

/// Parse `tt env stop ...`
fn env_stop<'a>(m: &'a ArgMatches<'a>) -> Result<EnvStop<'a>> {
    if let Some(env_set) = m.get_many::<String>("ENV") {
        Ok(EnvStop {
            env_set: env_set.cloned().collect(),
        })
    } else {
        Err(eg!("ENV not specified"))
    }
}

/// Parse `tt env start ...`
fn env_start<'a>(m: &'a ArgMatches<'a>) -> Result<EnvStart<'a>> {
    if let Some(env_set) = m.get_many::<String>("ENV") {
        Ok(EnvStart {
            env_set: env_set.cloned().collect(),
        })
    } else {
        Err(eg!("ENV not specified"))
    }
}

/// Parse `tt env show ...`
fn env_show<'a>(m: &'a ArgMatches<'a>) -> Result<EnvShow<'a>> {
    if let Some(env_set) = m.get_many::<String>("ENV") {
        Ok(EnvShow {
            env_set: env_set.cloned().collect(),
        })
    } else {
        Err(eg!("ENV not specified"))
    }
}

/// Parse `tt env update ...`
fn env_update<'a>(m: &'a ArgMatches<'a>) -> Result<EnvUpdate<'a>> {
    match (
        m.get_one::<String>("life-time"),
        m.get_many::<String>("kick-vm"),
        m.get_many::<String>("kick-os"),
        m.get_one::<String>("cpu-num"),
        m.get_one::<String>("mem-size"),
        m.get_many::<String>("vm-port"),
        m.get_many::<String>("ENV"),
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
            env_set: env_set.cloned().collect(),
            kick_dead: m.get_flag("kick-dead"),
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
                .map(|os| os.cloned().collect())
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
            is_fucker: m.get_flag("SSSS"),
            deny_outgoing: if m.get_flag("deny-outgoing") {
                Some(true)
            } else if m.get_flag("allow-outgoing") {
                Some(false)
            } else {
                None
            },
        }),
    }
}

/// Parse `tt env get ...`
fn env_get<'a>(m: &'a ArgMatches<'a>) -> Result<EnvGet<'a>> {
    // Default 10s timeout
    const TIMEOUT: &str = "10";

    match (
        m.get_one::<String>("file-path"),
        m.get_one::<String>("time-out"),
        m.get_many::<String>("os-prefix"),
        m.get_many::<String>("vm-id"),
        m.get_many::<String>("ENV"),
    ) {
        (None, _, _, _, _) => Err(eg!("--file-path not specified")),
        (_, _, _, _, None) => Err(eg!("没有指定 ENV")),
        (Some(file_path), timeout, osset, idset, Some(env_set)) => {
            Ok(EnvGet {
                use_ssh: m.get_flag("use-ssh"),
                file_path,
                time_out: timeout.map(|s| s.as_str()).unwrap_or(TIMEOUT).parse::<u64>().c(d!())?,
                env_set: env_set.cloned().collect(),
                filter_os_prefix: osset
                    .map(|os| os.cloned().collect())
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

/// Parse `tt env push ...`
fn env_push<'a>(m: &'a ArgMatches<'a>) -> Result<EnvPush<'a>> {
    // Default 10s timeout
    const TIMEOUT: &str = "10";

    match (
        m.get_one::<String>("file-path"),
        m.get_one::<String>("time-out"),
        m.get_many::<String>("os-prefix"),
        m.get_many::<String>("vm-id"),
        m.get_many::<String>("ENV"),
    ) {
        (None, _, _, _, _) => Err(eg!("--file-path not specified")),
        (_, _, _, _, None) => Err(eg!("没有指定 ENV")),
        (Some(file_path), timeout, osset, idset, Some(env_set)) => {
            Ok(EnvPush {
                use_ssh: m.get_flag("use-ssh"),
                file_path: check_file_path(file_path).c(d!())?,
                time_out: timeout.map(|s| s.as_str()).unwrap_or(TIMEOUT).parse::<u64>().c(d!())?,
                env_set: env_set.cloned().collect(),
                filter_os_prefix: osset
                    .map(|os| os.cloned().collect())
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

/// Parse `tt env run ...`
fn env_run<'a>(m: &'a ArgMatches<'a>) -> Result<EnvRun<'a>> {
    // Default 3s timeout
    const TIMEOUT: &str = "3";

    if let Some(env_set) = m.get_many::<String>("ENV") {
        let filter_os_prefix = m
            .get_many::<String>("os-prefix")
            .map(|os| os.cloned().collect())
            .unwrap_or_default();
        let filter_vm_id = if let Some(idset) = m.get_many::<String>("vm-id") {
            let mut res = vct![];
            for id in idset.into_iter() {
                res.push(id.parse::<i32>().c(d!(not_num!(id)))?);
            }
            res
        } else {
            vct![]
        };
        let to = m
            .get_one::<String>("time-out")
            .map(|s| s.as_str())
            .unwrap_or(TIMEOUT)
            .parse::<u64>()
            .c(d!())?;

        let env_set = env_set.cloned().collect();
        if m.get_flag("config-hawk") {
            Ok(EnvRun {
                cmd: "",
                script: "",
                time_out: to,
                env_set,
                use_ssh: m.get_flag("use-ssh"),
                interactive: false,
                config_hawk: true,
                filter_vm_id,
                filter_os_prefix,
            })
        } else if m.get_flag("interactive") {
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
            match (m.get_one::<String>("cmd"), m.get_one::<String>("script")) {
                (None, None) => Err(eg!("Neither --cmd nor --script specified")),
                (cmd, Some(script)) => Ok(EnvRun {
                    cmd: cmd.map(|s| s.as_str()).unwrap_or(""),
                    script: check_file_path(script).c(d!())?,
                    time_out: to,
                    env_set,
                    use_ssh: m.get_flag("use-ssh"),
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
                    use_ssh: m.get_flag("use-ssh"),
                    interactive: false,
                    config_hawk: false,
                    filter_vm_id,
                    filter_os_prefix,
                }),
            }
        }
    } else {
        Err(eg!("ENV not specified"))
    }
}

/// Parse `tt config ...`
fn config<'a>(m: &'a ArgMatches<'a>) -> Result<Config<'a>> {
    match (
        m.value_of("server-addr"),
        m.value_of("server-port"),
        m.value_of("client-id"),
    ) {
        (None, _, None) => Err(eg!("Neither --server-addr nor --client-id specified")),
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

/// Confirm that the target is a file
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
