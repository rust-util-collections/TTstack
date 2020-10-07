//!
//! # A client-end implementation.
//!

use clap::{
    crate_authors, crate_description, crate_name, crate_version, App,
    ArgMatches, SubCommand,
};
use myutil::{err::*, *};
use ttrexec::{
    client::{req_exec, req_transfer},
    common::{Direction, TransReq},
};

fn main() {
    let m = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommands(vct![
            SubCommand::with_name("exec")
                .arg_from_usage("-a, --server-addr=[ADDR] '服务端的监听地址.'")
                .arg_from_usage("-p, --server-port=[PORT] '服务端的监听端口.'")
                .arg_from_usage("-c, --remote-cmd=[CMD] '待执行的远程命令.'")
                .arg_from_usage("-t, --time-out=[TIME] '执行超时时间.'"),
            SubCommand::with_name("push")
                .arg_from_usage("-a, --server-addr=[ADDR] '服务端的监听地址.'")
                .arg_from_usage("-p, --server-port=[PORT] '服务端的监听端口.'")
                .arg_from_usage(
                    "-l, --local-path=[PATH] '客户端本地文件路径.'"
                )
                .arg_from_usage("-r, --remote-path=[PATH] '服务端文件路径.'")
                .arg_from_usage("-t, --time-out=[TIME] '执行超时时间.'"),
            SubCommand::with_name("get")
                .arg_from_usage("-a, --server-addr=[ADDR] '服务端的监听地址.'")
                .arg_from_usage("-p, --server-port=[PORT] '服务端的监听端口.'")
                .arg_from_usage(
                    "-l, --local-path=[PATH] '客户端本地文件路径.'"
                )
                .arg_from_usage("-r, --remote-path=[PATH] '服务端文件路径.'")
                .arg_from_usage("-t, --time-out=[TIME] '执行超时时间.'"),
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
                $m.value_of("server-addr"),
                $m.value_of("server-port"),
                $m.value_of("local-path"),
                $m.value_of("remote-path"),
                $m.value_of("time-out"),
            ) {
                (
                    Some(addr),
                    port,
                    Some(local_path),
                    Some(remote_path),
                    time_out,
                ) => {
                    let servaddr =
                        format!("{}:{}", addr, port.unwrap_or("22000"));
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
                exec_m.value_of("server-addr"),
                exec_m.value_of("server-port"),
                exec_m.value_of("remote-cmd"),
            ) {
                (Some(addr), port, Some(cmd)) => req_exec(
                    &format!("{}:{}", addr, port.unwrap_or("22000")),
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
