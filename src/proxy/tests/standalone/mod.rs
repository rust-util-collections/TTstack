use super::env::*;
use myutil::{err::*, *};
use std::collections::HashMap;
use ttserver_def::*;

const CUSTOM_CLI_ID: &str = "ErHa";

// 孤立测试每一个接口
pub(super) fn test() {
    // UDP protocol tests
    t_register_client_id(send_req);
    t_get_server_info(send_req);
    t_get_env_list(send_req);
    t_get_env_info(send_req);
    t_add_env(send_req);
    t_update_env_lifetime(send_req);
    t_update_env_kick_vm(send_req);
    t_del_env(send_req);

    // HTTP/TCP protocol tests
    t_register_client_id(send_req_http);
    t_get_server_info(send_req_http);
    t_get_env_list(send_req_http);
    t_get_env_info(send_req_http);
    t_add_env(send_req_http);
    t_update_env_lifetime(send_req_http);
    t_update_env_kick_vm(send_req_http);
    t_del_env(send_req_http);
}

fn t_register_client_id(send_req: Sender<&str>) {
    assert!(
        send_req(
            "register_client_id",
            Req::new(0, CUSTOM_CLI_ID.to_owned(), "")
        )
        .is_ok()
    );
    assert!(
        send_req(
            "register_client_id",
            Req::new(0, CUSTOM_CLI_ID.to_owned(), "")
        )
        .is_ok()
    );
}

fn t_get_server_info(send_req: Sender<&str>) {
    let uuid = 1;
    let resp = pnk!(send_req(
        "get_server_info",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), "")
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(serde_json::from_slice::<
        HashMap<String, RespGetServerInfo>,
    >(&resp.msg));
    assert_eq!(2, body.len());

    let mut body = body.into_iter().map(|(_, v)| v).fold(
        RespGetServerInfo::default(),
        |mut base, mut new| {
            base.vm_total += new.vm_total;
            base.cpu_total += new.cpu_total;
            base.mem_total += new.mem_total;
            base.disk_total += new.disk_total;
            base.cpu_used += new.cpu_used;
            base.mem_used += new.mem_used;
            base.disk_used += new.disk_used;
            base.supported_list.append(&mut new.supported_list);
            base
        },
    );
    body.supported_list.sort();
    body.supported_list.dedup();

    assert_eq!(body.vm_total, 0);
    assert_eq!(body.cpu_total, 2 * CPU_TOTAL);
    assert_eq!(body.mem_total, 2 * MEM_TOTAL);
    assert_eq!(body.disk_total, 2 * DISK_TOTAL);
    assert_eq!(body.cpu_used, 0);
    assert_eq!(body.mem_used, 0);
    assert_eq!(body.disk_used, 0);
    assert!(!body.supported_list.is_empty());
}

// 在 add_env 之前调用
fn t_get_env_list(send_req: Sender<&str>) {
    let uuid = 2;

    let resp = pnk!(send_req(
        "get_env_list",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), "")
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(
        serde_json::from_slice::<HashMap<String, RespGetEnvList>>(&resp.msg)
    );
    assert_eq!(2, body.len());

    body.iter().for_each(|b| {
        assert!(b.1.is_empty());
    });
}

fn t_get_env_info(send_req: Sender<ReqGetEnvInfo>) {
    let uuid = 3;

    let msg = ReqGetEnvInfo {
        env_set: vct!["abcxxx".to_owned(), "xxxabc".to_owned()],
    };

    let resp = pnk!(send_req(
        "get_env_info",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(
        serde_json::from_slice::<HashMap<String, RespGetEnvInfo>>(&resp.msg)
    );

    // 参见 get_env_info 的实现
    assert_eq!(0, body.len());
}

fn t_add_env(send_req: Sender<ReqAddEnv>) {
    let uuid = 4;

    let msg = ReqAddEnv {
        env_id: "UselessEnv".to_owned(),
        os_prefix: vct!["c".to_owned(), "u".to_owned()],
        life_time: None,
        cpu_num: None,
        mem_size: Some(512),
        disk_size: None,
        port_set: vct![],
        dup_each: None,
        deny_outgoing: false,
        rand_uuid: true,
        vmcfg: None,
    };

    let resp = pnk!(send_req(
        "add_env",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(serde_json::from_slice::<String>(&resp.msg));
    assert_eq!("Success!", &body);
}

fn t_update_env_lifetime(send_req: Sender<ReqUpdateEnvLife>) {
    let uuid = 5;

    let msg = ReqUpdateEnvLife {
        env_id: "UselessEnv".to_owned(),
        life_time: 88888888888888,
        is_fucker: true,
    };

    let resp = pnk!(send_req(
        "update_env_lifetime",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);
}

fn t_update_env_kick_vm(send_req: Sender<ReqUpdateEnvKickVm>) {
    let uuid = 6;

    let msg = ReqUpdateEnvKickVm {
        env_id: "UselessEnv".to_owned(),
        vm_id: vct![],
        os_prefix: vct!["c".to_owned(), "u".to_owned()],
    };

    let resp = pnk!(send_req(
        "update_env_kick_vm",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(serde_json::from_slice::<String>(&resp.msg));
    assert_eq!("Success!", &body);
}

fn t_del_env(send_req: Sender<ReqDelEnv>) {
    let uuid = 7;

    let msg = ReqDelEnv {
        env_id: "UselessEnv".to_owned(),
    };

    let resp = pnk!(send_req(
        "del_env",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(serde_json::from_slice::<String>(&resp.msg));
    assert_eq!("Success!", &body);
}
