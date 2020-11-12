use super::env::*;
use myutil::{err::*, *};
use std::collections::HashMap;
use ttserver_def::*;

const CUSTOM_CLI_ID: &str = "ErHa";

// 孤立测试每一个接口
pub(super) fn test() {
    t_register_client_id();
    t_get_server_info();
    t_get_env_list();
    t_get_env_info();
    t_add_env();
    t_del_env();
    t_update_env_lifetime();
    t_update_env_kick_vm();
}

fn t_register_client_id() {
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

fn t_get_server_info() {
    let uuid = 1111;
    let resp = pnk!(send_req(
        "get_server_info",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), "")
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(serde_json::from_slice::<
        HashMap<String, RespGetServerInfo>,
    >(&resp.msg));
    assert_eq!(1, body.len());

    let body = body.into_iter().next().unwrap().1;
    assert_eq!(body.vm_total, 0);
    assert_eq!(body.cpu_total, CPU_TOTAL);
    assert_eq!(body.mem_total, MEM_TOTAL);
    assert_eq!(body.disk_total, DISK_TOTAL);
    assert_eq!(body.cpu_used, 0);
    assert_eq!(body.mem_used, 0);
    assert_eq!(body.disk_used, 0);
    assert!(!body.supported_list.is_empty());
}

// 在 add_env 之前调用
fn t_get_env_list() {
    let uuid = 1111;

    let resp = pnk!(send_req(
        "get_env_list",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), "")
    ));

    assert_eq!(resp.uuid, uuid);
    assert_eq!(resp.status, RetStatus::Success);

    let body = pnk!(
        serde_json::from_slice::<HashMap<String, RespGetEnvList>>(&resp.msg)
    );
    assert_eq!(1, body.len());

    let body = body.into_iter().next().unwrap().1;
    assert!(body.is_empty());
}

fn t_get_env_info() {
    let uuid = 1111;

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
    assert_eq!(1, body.len());

    let body = body.into_iter().next().unwrap().1;
    assert!(body.is_empty());
}

fn t_add_env() {
    let uuid = 1111;

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

fn t_del_env() {
    let uuid = 1111;

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

fn t_update_env_lifetime() {
    let uuid = 1111;

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
    assert_eq!(resp.status, RetStatus::Fail);
}

fn t_update_env_kick_vm() {
    let uuid = 1111;

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
