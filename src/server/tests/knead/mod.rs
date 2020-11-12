//!
//! #  knead tests
//!
//! 按一定的逻辑组合各接口进行集成测试.
//!

use super::env::*;
use myutil::{err::*, *};
use std::collections::HashMap;
use ttserver_def::*;

const ENV_OK: &str = "This will ok!";
const ENV_FAIL: &str = "This will fail!";

const CUSTOM_CLI_ID: &str = "HaEr";

// 1. 创建两个 ENV, 第一个成功, 第二个失败(触发资源不足)
// 2. 核对系统剩余可用资源的正确性
// 3. 查询 ENV 列表, 应只获取一条信息, 且内容与预期的一致
// 4. 查询创建成功的那个 ENV 的详情, 核对其内容与预期一致
// 5. 从其中 Kick 出两个 VM, 核对剩余 VM 列表是否与预期一致
// 6. Kick 出剩余所有 VM, ENV 应依然存在
//     - ENV 的列表查询接口依然返回一条数据
//     - ENV 的详情查询接口依然能成功获取数据
//     - 更新其生命周期依然返回成功
// 7. 调用 del_env 接口返回成功
//     - ENV 的列表查询接口返回空
//     - ENV 的详情查询接口失败
//     - 更新生命周期接口失败
// 8. 核对系统剩余可用资源的正确性
pub(super) fn test() {
    let orig_server_info = get_server_info();

    let resp = add_env(ENV_OK, &["c", "xxa", "--b"], 1);
    assert_eq!(resp.status, RetStatus::Success);

    let resp = add_env(ENV_FAIL, &["c", "xxa", "--b"], 10000);
    assert_eq!(resp.status, RetStatus::Fail);

    let new_server_info = get_server_info();
    assert_eq!(orig_server_info.cpu_total, new_server_info.cpu_total);
    assert_eq!(
        new_server_info.cpu_used,
        new_server_info.supported_list.len() as i32
    );

    let env_list = get_env_list();
    assert_eq!(env_list.len(), 1);
    assert_eq!(&env_list[0].id, ENV_OK);
    assert_eq!(env_list[0].vm_cnt, new_server_info.supported_list.len());

    assert_eq!(
        update_life(ENV_OK, 99_0000, true).status,
        RetStatus::Success
    );
    let env_list = get_env_list();
    assert_eq!(
        env_list[0].end_timestamp,
        env_list[0].start_timestamp + 99_0000
    );

    assert!(pnk!(get_env_info(ENV_FAIL)).is_empty());

    let env_info = pnk!(get_env_info(ENV_OK));
    assert_eq!(env_info.len(), 1);
    assert_eq!(env_info[0].id, ENV_OK);
    assert_eq!(env_info[0].vm.len(), new_server_info.supported_list.len());

    assert_eq!(
        kick_vm(ENV_OK, &["centos7.1", "centos7.2"]).status,
        RetStatus::Success
    );

    let env_list = get_env_list();
    assert_eq!(env_list.len(), 1);
    assert_eq!(&env_list[0].id, ENV_OK);
    assert_eq!(env_list[0].vm_cnt, new_server_info.supported_list.len() - 2);

    assert_eq!(update_life(ENV_OK, 99, true).status, RetStatus::Success);
    let env_list = get_env_list();
    assert_eq!(env_list[0].end_timestamp, env_list[0].start_timestamp + 99);

    let env_info = pnk!(get_env_info(ENV_OK));
    assert_eq!(env_info.len(), 1);
    assert_eq!(env_info[0].id, ENV_OK);
    assert_eq!(
        env_info[0].vm.len(),
        new_server_info.supported_list.len() - 2
    );

    assert_eq!(kick_vm(ENV_OK, &["c"]).status, RetStatus::Success);

    let env_list = get_env_list();
    assert_eq!(env_list.len(), 1);
    assert_eq!(&env_list[0].id, ENV_OK);
    assert_eq!(env_list[0].vm_cnt, 0);

    assert_eq!(update_life(ENV_OK, 199, true).status, RetStatus::Success);
    let env_list = get_env_list();
    assert_eq!(env_list[0].end_timestamp, env_list[0].start_timestamp + 199);

    let env_info = pnk!(get_env_info(ENV_OK));
    assert_eq!(env_info.len(), 1);
    assert_eq!(env_info[0].id, ENV_OK);
    assert_eq!(env_info[0].vm.len(), 0);

    del_env(ENV_OK);

    let env_list = get_env_list();
    assert!(env_list.is_empty());

    assert!(pnk!(get_env_info(ENV_OK)).is_empty());

    assert_eq!(update_life(ENV_OK, 9, true).status, RetStatus::Fail);

    let last_server_info = get_server_info();
    assert_eq!(orig_server_info, last_server_info);
}

fn get_server_info() -> RespGetServerInfo {
    let uuid = 5566;
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
    assert!(!body.supported_list.is_empty());

    body
}

// 在 add_env 之前调用
fn get_env_list() -> RespGetEnvList {
    let uuid = 5566;

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
    body
}

fn get_env_info(env_id: &str) -> Result<RespGetEnvInfo> {
    let uuid = 5566;
    let msg = ReqGetEnvInfo {
        env_set: vct![env_id.to_owned()],
    };

    let resp = pnk!(send_req(
        "get_env_info",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));
    assert_eq!(resp.uuid, uuid);

    if resp.status == RetStatus::Success {
        let body = pnk!(serde_json::from_slice::<
            HashMap<String, RespGetEnvInfo>,
        >(&resp.msg));
        assert_eq!(1, body.len());

        let body = body.into_iter().next().unwrap().1;
        Ok(body)
    } else {
        Err(eg!())
    }
}

fn add_env(env_id: &str, os_prefix: &[&str], cpu_num: i32) -> Resp {
    let uuid = 5566;
    let msg = ReqAddEnv {
        env_id: env_id.to_owned(),
        os_prefix: os_prefix
            .iter()
            .map(|os| os.to_string())
            .collect::<Vec<_>>(),
        life_time: None,
        cpu_num: Some(cpu_num),
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

    resp
}

fn del_env(env_id: &str) {
    let uuid = 5566;
    let msg = ReqDelEnv {
        env_id: env_id.to_owned(),
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

fn update_life(env_id: &str, life: u64, is_fucker: bool) -> Resp {
    let uuid = 5566;
    let msg = ReqUpdateEnvLife {
        env_id: env_id.to_owned(),
        life_time: life,
        is_fucker,
    };

    let resp = pnk!(send_req(
        "update_env_lifetime",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));
    assert_eq!(resp.uuid, uuid);

    resp
}

fn kick_vm(env_id: &str, os_prefix: &[&str]) -> Resp {
    let uuid = 5566;
    let msg = ReqUpdateEnvKickVm {
        env_id: env_id.to_owned(),
        vm_id: vct![],
        os_prefix: os_prefix
            .iter()
            .map(|os| os.to_string())
            .collect::<Vec<_>>(),
    };

    let resp = pnk!(send_req(
        "update_env_kick_vm",
        Req::new(uuid, CUSTOM_CLI_ID.to_owned(), msg)
    ));
    assert_eq!(resp.uuid, uuid);

    resp
}
