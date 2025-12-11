//! HTTP API handlers for the central controller.
//!
//! Handles requests from the CLI and coordinates with host agents.

use crate::db::Db;
use crate::scheduler;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::{Arc, Mutex};
use ttcore::api::*;
use ttcore::model::*;

/// Shared controller state.
pub type CtlState = Arc<Mutex<Db>>;

/// HTTP client for agent communication.
fn agent_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

// ── Host Management ─────────────────────────────────────────────────

/// POST /api/hosts — register a new host by its agent address.
pub async fn register_host(
    State(db): State<CtlState>,
    Json(req): Json<RegisterHostReq>,
) -> impl IntoResponse {
    let client = agent_client();
    let url = format!("http://{}/api/info", req.addr);

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResp::<Host>::err(format!(
                    "cannot reach agent at {}: {e}",
                    req.addr
                ))),
            );
        }
    };

    let info: ApiResp<AgentInfo> = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResp::<Host>::err(format!("invalid agent response: {e}"))),
            );
        }
    };

    let info = match info.data {
        Some(i) => i,
        None => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResp::<Host>::err("agent returned no data")),
            );
        }
    };

    let db = db.lock().unwrap();

    if db.host_count().unwrap_or(0) >= MAX_HOSTS {
        return (
            StatusCode::CONFLICT,
            Json(ApiResp::<Host>::err(format!(
                "fleet limit reached ({MAX_HOSTS} hosts)"
            ))),
        );
    }

    let host = Host {
        id: info.host_id,
        addr: req.addr,
        resource: info.resource,
        state: HostState::Online,
        engines: info.engines,
        storage: info.storage,
        registered_at: now(),
    };

    if let Err(e) = db.put_host(&host) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResp::<Host>::err(e.to_string())),
        );
    }

    (StatusCode::CREATED, Json(ApiResp::success(host)))
}

/// GET /api/hosts
pub async fn list_hosts(State(db): State<CtlState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.list_hosts() {
        Ok(hosts) => Json(ApiResp::success(hosts)),
        Err(e) => Json(ApiResp::<Vec<Host>>::err(e.to_string())),
    }
}

/// GET /api/hosts/:id
pub async fn get_host(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_host(&id) {
        Ok(Some(h)) => (StatusCode::OK, Json(ApiResp::success(h))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResp::<Host>::err(format!("host not found: {id}"))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResp::<Host>::err(e.to_string())),
        ),
    }
}

/// DELETE /api/hosts/:id
pub async fn remove_host(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = db.lock().unwrap();

    let vms = db.vms_by_host(&id).unwrap_or_default();
    if !vms.is_empty() {
        return (
            StatusCode::CONFLICT,
            Json(ApiRespEmpty::err(format!(
                "host {id} still has {} VMs; destroy them first",
                vms.len()
            ))),
        );
    }

    if let Err(e) = db.remove_host(&id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiRespEmpty::err(e.to_string())),
        );
    }

    (StatusCode::OK, Json(ApiRespEmpty::ok()))
}

// ── Environment Management ──────────────────────────────────────────

/// POST /api/envs — create an environment with VMs.
pub async fn create_env(
    State(db): State<CtlState>,
    Json(req): Json<CreateEnvReq>,
) -> impl IntoResponse {
    // Validate & schedule under lock
    let hosts = {
        let db = db.lock().unwrap();

        if let Ok(Some(_)) = db.get_env(&req.id) {
            return (
                StatusCode::CONFLICT,
                Json(ApiResp::<EnvDetail>::err(format!(
                    "environment '{}' already exists",
                    req.id
                ))),
            );
        }

        match db.list_hosts() {
            Ok(h) => h,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResp::<EnvDetail>::err(e.to_string())),
                );
            }
        }
    };

    let placements = match scheduler::schedule_env(&hosts, &req.vms) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResp::<EnvDetail>::err(e.to_string())),
            );
        }
    };

    let created_at = now();
    let expires_at = req
        .lifetime
        .map(|lt| created_at + lt.min(MAX_LIFETIME))
        .unwrap_or(created_at + MAX_LIFETIME);

    let client = agent_client();
    let mut vm_ids = Vec::new();
    let mut created_vms = Vec::new();

    // Create VMs on agents (no lock held during HTTP calls)
    for (spec, placement) in &placements {
        let vm_id = uuid::Uuid::new_v4().to_string()[..12].to_string();
        let cpu = spec.cpu.unwrap_or(VM_CPU_DEFAULT);
        let mem = spec.mem.unwrap_or(VM_MEM_DEFAULT);
        let disk = spec.disk.unwrap_or(VM_DISK_DEFAULT);

        let agent_req = CreateVmReq {
            vm_id: vm_id.clone(),
            env_id: req.id.clone(),
            image: spec.image.clone(),
            engine: spec.engine,
            cpu,
            mem,
            disk,
            ports: spec.ports.clone(),
            deny_outgoing: spec.deny_outgoing,
        };

        let url = format!("http://{}/api/vms", placement.host_addr);
        match client.post(&url).json(&agent_req).send().await {
            Ok(r) if r.status().is_success() => {
                if let Ok(body) = r.json::<ApiResp<CreateVmResp>>().await
                    && let Some(data) = body.data {
                        vm_ids.push(vm_id);
                        created_vms.push(data.vm);
                        continue;
                    }
                eprintln!(
                    "warning: unparseable agent response for VM on {}",
                    placement.host_addr
                );
            }
            Ok(r) => {
                eprintln!(
                    "warning: agent returned {} for VM on {}",
                    r.status(),
                    placement.host_addr
                );
            }
            Err(e) => {
                eprintln!(
                    "warning: failed to create VM on {}: {e}",
                    placement.host_addr
                );
            }
        }
    }

    let env = Env {
        id: req.id.clone(),
        owner: req.owner.clone(),
        vm_ids: vm_ids.clone(),
        created_at,
        expires_at,
        state: EnvState::Active,
    };

    // Persist under lock
    {
        let db = db.lock().unwrap();
        let _ = db.put_env(&env);
        for vm in &created_vms {
            let _ = db.put_vm(vm);
        }
        // Refresh host resources
        drop(db);
    }

    refresh_all_hosts(&db, &client).await;

    let detail = EnvDetail {
        env,
        vms: created_vms,
    };

    (StatusCode::CREATED, Json(ApiResp::success(detail)))
}

/// GET /api/envs
pub async fn list_envs(State(db): State<CtlState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.list_envs() {
        Ok(envs) => Json(ApiResp::success(envs)),
        Err(e) => Json(ApiResp::<Vec<Env>>::err(e.to_string())),
    }
}

/// GET /api/envs/:id
pub async fn get_env(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_env(&id) {
        Ok(Some(env)) => {
            let vms = db.vms_by_env(&id).unwrap_or_default();
            (
                StatusCode::OK,
                Json(ApiResp::success(EnvDetail { env, vms })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResp::<EnvDetail>::err(format!(
                "environment not found: {id}"
            ))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResp::<EnvDetail>::err(e.to_string())),
        ),
    }
}

/// DELETE /api/envs/:id
pub async fn delete_env(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (vms, hosts) = {
        let db = db.lock().unwrap();
        match db.get_env(&id) {
            Ok(Some(_)) => {}
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiRespEmpty::err(format!("environment not found: {id}"))),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiRespEmpty::err(e.to_string())),
                );
            }
        };
        let vms = db.vms_by_env(&id).unwrap_or_default();
        let hosts = db.list_hosts().unwrap_or_default();
        (vms, hosts)
    };

    let client = agent_client();
    for vm in &vms {
        if let Some(host) = hosts.iter().find(|h| h.id == vm.host_id) {
            let url = format!("http://{}/api/vms/{}", host.addr, vm.id);
            let _ = client.delete(&url).send().await;
        }
    }

    {
        let db = db.lock().unwrap();
        for vm in &vms {
            let _ = db.remove_vm(&vm.id);
        }
        let _ = db.remove_env(&id);
    }

    (StatusCode::OK, Json(ApiRespEmpty::ok()))
}

/// POST /api/envs/:id/stop
pub async fn stop_env(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (mut env, vms, hosts) = {
        let db = db.lock().unwrap();
        let env = match db.get_env(&id) {
            Ok(Some(e)) => e,
            _ => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiRespEmpty::err(format!("environment not found: {id}"))),
                );
            }
        };
        let vms = db.vms_by_env(&id).unwrap_or_default();
        let hosts = db.list_hosts().unwrap_or_default();
        (env, vms, hosts)
    };

    let client = agent_client();
    for vm in &vms {
        if let Some(host) = hosts.iter().find(|h| h.id == vm.host_id) {
            let url = format!("http://{}/api/vms/{}/stop", host.addr, vm.id);
            let _ = client.post(&url).send().await;
        }
    }

    env.state = EnvState::Stopped;
    let db = db.lock().unwrap();
    let _ = db.put_env(&env);

    (StatusCode::OK, Json(ApiRespEmpty::ok()))
}

/// POST /api/envs/:id/start
pub async fn start_env(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (mut env, vms, hosts) = {
        let db = db.lock().unwrap();
        let env = match db.get_env(&id) {
            Ok(Some(e)) => e,
            _ => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiRespEmpty::err(format!("environment not found: {id}"))),
                );
            }
        };
        let vms = db.vms_by_env(&id).unwrap_or_default();
        let hosts = db.list_hosts().unwrap_or_default();
        (env, vms, hosts)
    };

    let client = agent_client();
    for vm in &vms {
        if let Some(host) = hosts.iter().find(|h| h.id == vm.host_id) {
            let url = format!("http://{}/api/vms/{}/start", host.addr, vm.id);
            let _ = client.post(&url).send().await;
        }
    }

    env.state = EnvState::Active;
    let db = db.lock().unwrap();
    let _ = db.put_env(&env);

    (StatusCode::OK, Json(ApiRespEmpty::ok()))
}

// ── Images ──────────────────────────────────────────────────────────

/// GET /api/images
pub async fn list_images(State(db): State<CtlState>) -> impl IntoResponse {
    let hosts = {
        let db = db.lock().unwrap();
        db.list_hosts().unwrap_or_default()
    };

    let client = agent_client();
    let mut images = Vec::new();

    for host in &hosts {
        if host.state != HostState::Online {
            continue;
        }
        let url = format!("http://{}/api/images", host.addr);
        if let Ok(resp) = client.get(&url).send().await
            && let Ok(body) = resp.json::<ApiResp<Vec<String>>>().await
                && let Some(names) = body.data {
                    for name in names {
                        images.push(ImageInfo {
                            name,
                            host_id: host.id.clone(),
                        });
                    }
                }
    }

    Json(ApiResp::success(images))
}

// ── Status ──────────────────────────────────────────────────────────

/// GET /api/status
pub async fn fleet_status(State(db): State<CtlState>) -> impl IntoResponse {
    let client = agent_client();
    refresh_all_hosts(&db, &client).await;

    let db = db.lock().unwrap();
    match db.fleet_status() {
        Ok(s) => Json(ApiResp::success(s)),
        Err(e) => Json(ApiResp::<FleetStatus>::err(e.to_string())),
    }
}

// ── VM Lookup ───────────────────────────────────────────────────────

/// GET /api/vms/:id — get a single VM by ID (across all hosts).
pub async fn get_vm(
    State(db): State<CtlState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_vm(&id) {
        Ok(Some(vm)) => (StatusCode::OK, Json(ApiResp::success(vm))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResp::<Vm>::err(format!("VM not found: {id}"))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResp::<Vm>::err(e.to_string())),
        ),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Refresh resource snapshots for all hosts from their agents.
async fn refresh_all_hosts(state: &CtlState, client: &reqwest::Client) {
    let hosts = {
        let db = state.lock().unwrap();
        db.list_hosts().unwrap_or_default()
    };

    for host in &hosts {
        let url = format!("http://{}/api/info", host.addr);
        let mut updated = host.clone();

        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.json::<ApiResp<AgentInfo>>().await
                    && let Some(info) = body.data {
                        updated.resource = info.resource;
                        updated.state = HostState::Online;
                    }
            }
            Err(_) => {
                updated.state = HostState::Offline;
            }
        }

        let db = state.lock().unwrap();
        let _ = db.put_host(&updated);
    }
}
