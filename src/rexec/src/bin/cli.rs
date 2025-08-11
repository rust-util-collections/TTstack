//!
//! # A client-end implementation.
//!

use clap::{Arg, ArgMatches, Command};
use ruc::*;
use ttrexec::{
    client::{req_exec, req_transfer},
    common::{Direction, TransReq},
};

fn main() {
    let m = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommands(vct![
            Command::new("exec")
                .arg(Arg::new("server-addr")
                    .short('a')
                    .long("server-addr")
                    .value_name("ADDR")
                    .help("服务端的监听地址."))
                .arg(Arg::new("server-port")
                    .short('p')
                    .long("server-port")
                    .value_name("PORT")
                    .help("服务端的监听端口."))
                .arg(Arg::new("remote-cmd")
                    .short('c')
                    .long("remote-cmd")
                    .value_name("CMD")
                    .help("待执行的远程命令."))
                .arg(Arg::new("time-out")
                    .short('t')
                    .long("time-out")
                    .value_name("TIME")
                    .help("执行超时时间.")),
            Command::new("push")
                .arg(Arg::new("server-addr")
                    .short('a')
                    .long("server-addr")
                    .value_name("ADDR")
                    .help("服务端的监听地址."))
                .arg(Arg::new("server-port")
                    .short('p')
                    .long("server-port")
                    .value_name("PORT")
                    .help("服务端的监听端口."))
                .arg(Arg::new("local-path")
                    .short('l')
                    .long("local-path")
                    .value_name("PATH")
                    .help("客户端本地文件路径."))
                .arg(Arg::new("remote-path")
                    .short('r')
                    .long("remote-path")
                    .value_name("PATH")
                    .help("服务端文件路径."))
                .arg(Arg::new("time-out")
                    .short('t')
                    .long("time-out")
                    .value_name("TIME")
                    .help("执行超时时间.")),
            Command::new("get")
                .arg(Arg::new("server-addr")
                    .short('a')
                    .long("server-addr")
                    .value_name("ADDR")
                    .help("服务端的监听地址."))
                .arg(Arg::new("server-port")
                    .short('p')
                    .long("server-port")
                    .value_name("PORT")
                    .help("服务端的监听端口."))
                .arg(Arg::new("local-path")
                    .short('l')
                    .long("local-path")
                    .value_name("PATH")
                    .help("客户端本地文件路径."))
                .arg(Arg::new("remote-path")
                    .short('r')
                    .long("remote-path")
                    .value_name("PATH")
                    .help("服务端文件路径."))
                .arg(Arg::new("time-out")
                    .short('t')
                    .long("time-out")
                    .value_name("TIME")
                    .help("执行超时时间.")),
        ])
        .get_matches();

    pnk!(parse_and_exec(m));
}

fn parse_and_exec(m: ArgMatches) -> Result<()> {
    macro_rules! err {
        () => {
            return Err(eg!(format!("{:#?}", m)));
        };
    }

    macro_rules! trans {
        ($m: expr, $drct: tt) => {
            match (
                $m.get_one::<String>("server-addr"),
                $m.get_one::<String>("server-port"),
                $m.get_one::<String>("local-path"),
                $m.get_one::<String>("remote-path"),
                $m.get_one::<String>("time-out"),
            ) {
                (
                    Some(addr),
                    port,
                    Some(local_path),
                    Some(remote_path),
                    time_out,
                ) => {
                    let servaddr =
                        format!("{}:{}", addr, port.map(|s| s.as_str()).unwrap_or("22000"));
                    TransReq::new(Direction::$drct, local_path, remote_path)
                        .c(d!())
                        .and_then(|req| {
                            req_transfer(
                                &servaddr,
                                req,
                                Some(
                                    time_out
                                        .unwrap_or("3")
                                        .parse::<u64>()
                                        .c(d!())?,
                                ),
                            )
                            .c(d!())
                        })
                        .and_then(|resp| {
                            alt!(0 == dbg!(resp).code, Ok(()), err!())
                        })
                }
                _ => err!(),
            }
        };
    }

    match m.subcommand() {
        ("exec", Some(exec_m)) => {
            match (
                exec_m.get_one::<String>("server-addr"),
                exec_m.get_one::<String>("server-port"),
                exec_m.get_one::<String>("remote-cmd"),
            ) {
                (Some(addr), port, Some(cmd)) => req_exec(
                    &format!("{}:{}", addr, port.map(|s| s.as_str()).unwrap_or("22000")),
                    cmd,
                )
                .c(d!())
                .and_then(|resp| alt!(0 == dbg!(resp).code, Ok(()), err!())),
                _ => err!(),
            }
        }
        ("push", Some(push_m)) => trans!(push_m, Push),
        ("get", Some(get_m)) => trans!(get_m, Get),
        _ => err!(),
    }
}
