//! TTstack host agent entry point.
//!
//! The agent runs on each physical host, managing local VMs/containers
//! and exposing an HTTP API for the central controller.

mod config;
mod handler;
mod runtime;

use axum::Router;
use axum::routing::{get, post};
use clap::Parser;
use config::Config;
use handler::AppState;
use runtime::Runtime;
use std::sync::{Arc, Mutex};
use ttcore::model::Resource;

#[tokio::main]
async fn main() {
    let cfg = Config::parse();

    let host_id = cfg
        .host_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()[..8].to_string());

    let resource = Resource {
        cpu_total: cfg.effective_cpu(),
        mem_total: cfg.effective_mem(),
        disk_total: cfg.disk_total,
        ..Default::default()
    };

    let db_path = format!("{}/agent.db", cfg.data_dir);
    std::fs::create_dir_all(&cfg.data_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create data dir {}: {e}", cfg.data_dir);
        std::process::exit(1);
    });

    let rt = Runtime::new(
        host_id.clone(),
        cfg.storage_kind(),
        cfg.image_dir.clone(),
        cfg.runtime_dir.clone(),
        &db_path,
        resource,
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to initialize runtime: {e}");
        std::process::exit(1);
    });

    let state: AppState = Arc::new(Mutex::new(rt));

    let app = Router::new()
        .route("/api/info", get(handler::get_info))
        .route("/api/images", get(handler::list_images))
        .route("/api/vms", get(handler::list_vms).post(handler::create_vm))
        .route(
            "/api/vms/{id}",
            get(handler::get_vm).delete(handler::destroy_vm),
        )
        .route("/api/vms/{id}/stop", post(handler::stop_vm))
        .route("/api/vms/{id}/start", post(handler::start_vm))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.listen)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind {}: {e}", cfg.listen);
            std::process::exit(1);
        });

    eprintln!("tt-agent [{host_id}] listening on {}", cfg.listen);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| eprintln!("Server error: {e}"));

    eprintln!("tt-agent shutting down");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("received shutdown signal");
}
