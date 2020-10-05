use myutil::{err::*, *};
use std::{fs, path::Path, thread, time::Duration};
use ttrexec::{
    client,
    common::{Direction, TransReq},
    server,
};

const UDP_SERV_ADDR: &str = "127.0.0.1:49527";
const TCP_SERV_ADDR: &str = "127.0.0.1:49527";

#[test]
fn i_ttrexec() {
    start_server();
    do_client_ops();
}

fn start_server() {
    thread::spawn(|| {
        pnk!(server::serv_cmd(UDP_SERV_ADDR));
    });

    thread::spawn(|| {
        pnk!(server::serv_transfer(TCP_SERV_ADDR));
    });

    thread::sleep(Duration::from_secs(1));
}

fn do_client_ops() {
    exec_normal_success();
    exec_normal_fail();

    trans_normal_success_get();
    trans_normal_success_push();
    trans_normal_fail_get_client_err();
    trans_normal_fail_get_server_err();
    trans_normal_fail_push_client_err();
    trans_normal_fail_push_server_err();
}

fn exec_normal_success() {
    let resp = pnk!(client::req_exec(UDP_SERV_ADDR, "uname"));
    assert_eq!(0, resp.code);
    assert!(0 < resp.stdout.len());
    assert_eq!(0, resp.stderr.len());
}

fn exec_normal_fail() {
    let resp = pnk!(client::req_exec(UDP_SERV_ADDR, "ls /a;lfjkal;hjkf"));
    assert_ne!(0, resp.code);
    assert_eq!(0, resp.stdout.len());
    assert!(0 < resp.stderr.len());
}

fn trans_normal_success_get() {
    let local_file_path = "/tmp/passwd";
    let remote_file_path = "/etc/passwd";

    if Path::new(local_file_path).exists() {
        pnk!(fs::remove_file(local_file_path));
    }

    let req = pnk!(TransReq::new(
        Direction::Get,
        local_file_path,
        remote_file_path
    ));
    let resp = pnk!(client::req_transfer(TCP_SERV_ADDR, req, None));

    assert_eq!(0, resp.code);
    assert_eq!(0, resp.stderr.len());

    let contents_local = pnk!(fs::read(local_file_path));
    let contents_remote = pnk!(fs::read(remote_file_path));
    assert_eq!(contents_local, contents_remote);
}

fn trans_normal_success_push() {
    let local_file_path = "/etc/passwd";
    let remote_file_path = "/tmp/passwd";

    if Path::new(remote_file_path).exists() {
        pnk!(fs::remove_file(remote_file_path));
    }

    let req = pnk!(TransReq::new(
        Direction::Push,
        local_file_path,
        remote_file_path
    ));
    let resp = pnk!(client::req_transfer(TCP_SERV_ADDR, req, None));

    assert_eq!(0, resp.code);
    assert_eq!(0, resp.stderr.len());

    let contents_local = pnk!(fs::read(local_file_path));
    let contents_remote = pnk!(fs::read(remote_file_path));
    assert_eq!(contents_local, contents_remote);
}

fn trans_normal_fail_get_client_err() {
    let local_file_path = "/tmpppppppp/passwd";
    let remote_file_path = "/etc/passwd";

    if Path::new(local_file_path).exists() {
        pnk!(fs::remove_file(local_file_path));
    }

    let req = pnk!(TransReq::new(
        Direction::Get,
        local_file_path,
        remote_file_path
    ));

    assert!(client::req_transfer(TCP_SERV_ADDR, req, None).is_err());
}

fn trans_normal_fail_get_server_err() {
    let local_file_path = "/tmp/passwd";
    let remote_file_path = "/a;lkfjal;kfl;akjfl;ahgalkj";

    if Path::new(local_file_path).exists() {
        pnk!(fs::remove_file(local_file_path));
    }

    let req = pnk!(TransReq::new(
        Direction::Get,
        local_file_path,
        remote_file_path
    ));
    let resp = pnk!(client::req_transfer(TCP_SERV_ADDR, req, None));

    assert_ne!(0, resp.code);
    assert!(0 < resp.stderr.len());
}

fn trans_normal_fail_push_client_err() {
    let local_file_path = "/a;lkfjajkf";
    let remote_file_path = "/tmp/passwd";

    if Path::new(remote_file_path).exists() {
        pnk!(fs::remove_file(remote_file_path));
    }

    assert!(
        TransReq::new(Direction::Push, local_file_path, remote_file_path)
            .is_err()
    );
}

fn trans_normal_fail_push_server_err() {
    let local_file_path = "/etc/passwd";
    let remote_file_path = "";

    if Path::new(remote_file_path).exists() {
        pnk!(fs::remove_file(remote_file_path));
    }

    let req = pnk!(TransReq::new(
        Direction::Push,
        local_file_path,
        remote_file_path
    ));
    let resp = pnk!(client::req_transfer(TCP_SERV_ADDR, req, None));

    assert_ne!(0, resp.code);
    assert!(0 < resp.stderr.len());
}
