//!
//! # A server-end implementation.
//!
//! default listen at "0.0.0.0:22000", UDP and TCP.
//!

use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use myutil::{err::*, *};
use std::thread;
use ttrexec::server::{serv_cmd, serv_transfer};

fn main() {
    let m = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg_from_usage("-a, --server-addr=[ADDR] '服务端的监听地址.'")
        .arg_from_usage("-p, --server-port=[PORT] '服务端的监听端口.'")
        .get_matches();

    let servaddr = format!(
        "{}:{}",
        m.value_of("server-addr").unwrap_or("0.0.0.0"),
        m.value_of("server-port").unwrap_or("22000")
    );
    let servaddr1 = servaddr.clone();

    thread::spawn(move || {
        pnk!(serv_cmd(&servaddr));
    });

    pnk!(serv_transfer(&servaddr1));
}
