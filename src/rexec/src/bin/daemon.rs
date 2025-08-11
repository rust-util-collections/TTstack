//!
//! # A server-end implementation.
//!
//! default listen at "0.0.0.0:22000", UDP and TCP.
//!

use clap::{Arg, Command};
use ruc::*;
use std::thread;
use ttrexec::server::{serv_cmd, serv_transfer};

fn main() {
    let m = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
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
        .get_matches();

    let servaddr = format!(
        "{}:{}",
        m.get_one::<String>("server-addr").map(|s| s.as_str()).unwrap_or("0.0.0.0"),
        m.get_one::<String>("server-port").map(|s| s.as_str()).unwrap_or("22000")
    );
    let servaddr1 = servaddr.clone();

    thread::spawn(move || {
        pnk!(serv_cmd(&servaddr));
    });

    pnk!(serv_transfer(&servaddr1));
}
