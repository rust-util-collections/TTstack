//! HTTP API handlers for the host agent.

use crate::runtime::Runtime;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::{Arc, Mutex};
use ttcore::api::*;
use ttcore::model::Vm;

/// Shared application state.
pub type AppState = Arc<Mutex<Runtime>>;

/// GET /api/info — report host resources and capabilities.
pub async fn get_info(State(rt): State<AppState>) -> impl IntoResponse {
    let rt = rt.lock().unwrap();
    Json(ApiResp::success(rt.agent_info()))
}

/// GET /api/images — list available base images.
pub async fn list_images(State(rt): State<AppState>) -> impl IntoResponse {
    let rt = rt.lock().unwrap();
    Json(ApiResp::success(rt.list_images()))
}

/// POST /api/vms — create a new VM.
pub async fn create_vm(
    State(rt): State<AppState>,
    Json(req): Json<CreateVmReq>,
) -> impl IntoResponse {
    let mut rt = rt.lock().unwrap();
    match rt.create_vm(&req) {
        Ok(vm) => (StatusCode::CREATED, Json(ApiResp::success(CreateVmResp { vm }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResp::<CreateVmResp>::err(e.to_string())),
        ),
    }
}

/// GET /api/vms — list all VMs on this host.
pub async fn list_vms(State(rt): State<AppState>) -> impl IntoResponse {
    let rt = rt.lock().unwrap();
    Json(ApiResp::success(rt.list_vms()))
}

/// GET /api/vms/:id — get a specific VM.
pub async fn get_vm(
    State(rt): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let rt = rt.lock().unwrap();
    match rt.get_vm(&id) {
        Some(vm) => (StatusCode::OK, Json(ApiResp::success(vm))),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResp::<Vm>::err(format!("VM not found: {id}"))),
        ),
    }
}

/// DELETE /api/vms/:id — destroy a VM.
pub async fn destroy_vm(
    State(rt): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut rt = rt.lock().unwrap();
    match rt.destroy_vm(&id) {
        Ok(()) => (StatusCode::OK, Json(ApiRespEmpty::ok())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiRespEmpty::err(e.to_string())),
        ),
    }
}

/// POST /api/vms/:id/stop — stop a VM.
pub async fn stop_vm(
    State(rt): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut rt = rt.lock().unwrap();
    match rt.stop_vm(&id) {
        Ok(()) => (StatusCode::OK, Json(ApiRespEmpty::ok())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiRespEmpty::err(e.to_string())),
        ),
    }
}

/// POST /api/vms/:id/start — start a stopped VM.
pub async fn start_vm(
    State(rt): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut rt = rt.lock().unwrap();
    match rt.start_vm(&id) {
        Ok(()) => (StatusCode::OK, Json(ApiRespEmpty::ok())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiRespEmpty::err(e.to_string())),
        ),
    }
}
