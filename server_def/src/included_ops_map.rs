
/// 对应 Client 端的请求
pub(crate) const OPS_MAP: &[Ops] = &[
    /*0*/ register_client_id,
    /*1*/ get_server_info,
    /*2*/ get_env_list,
    /*3*/ get_env_info,
    /*4*/ add_env,
    /*5*/ del_env,
    /*6*/ update_env_lifetime,
    /*7*/ update_env_kick_vm,
    /*8*/ get_env_list_all,
    /*9*/ stop_env,
    /*10*/ start_env,
    /*11*/ update_env_resource,
];
