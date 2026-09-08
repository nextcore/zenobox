use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::compose::{compose_down, compose_up};
use crate::container::{
    container_create, container_delete, container_list_internal, container_logs, container_start, container_stop,
};
use crate::image::{list_images, pull_image};
use crate::network::list_networks;
use crate::utils::get_data_dir;
use crate::volume::list_volumes;

#[derive(Deserialize)]
pub struct NativeRunPayload {
    pub image: String,
    pub name: Option<String>,
    pub cmd: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub network: Option<String>,
    pub restart: Option<String>,
}

#[derive(Deserialize)]
pub struct ComposePayload {
    pub file_path: String,
}

pub fn native_router() -> Router {
    Router::new()
        .route("/api/v1/containers", get(native_list_containers).post(native_create_container))
        .route("/api/v1/containers/{id}/start", post(native_start_container))
        .route("/api/v1/containers/{id}/stop", post(native_stop_container))
        .route("/api/v1/containers/{id}", delete(native_delete_container))
        .route("/api/v1/containers/{id}/logs", get(native_get_logs))
        .route("/api/v1/containers/{id}/stats", get(native_get_stats))
        .route("/api/v1/containers/{id}/terminal", get(native_terminal_ws))
        .route("/api/v1/compose/up", post(native_compose_up))
        .route("/api/v1/compose/down", post(native_compose_down))
        .route("/api/v1/images", get(native_list_images))
        .route("/api/v1/images/pull", post(native_pull_image))
        .route("/api/v1/volumes", get(native_list_volumes))
        .route("/api/v1/networks", get(native_list_networks))
}

async fn native_list_containers() -> Json<serde_json::Value> {
    let data_dir = get_data_dir();
    let containers = container_list_internal(&data_dir, true).unwrap_or_default();
    Json(json!({ "containers": containers }))
}

async fn native_create_container(Json(payload): Json<NativeRunPayload>) -> Response {
    let name = payload.name.unwrap_or_else(|| format!("zeno-{}", rand::random::<u32>()));
    let cmd = payload.cmd.unwrap_or_default();
    let env = payload.env.unwrap_or_default();
    let ports = payload.ports.unwrap_or_default();
    let volumes = payload.volumes.unwrap_or_default();
    let network = payload.network.unwrap_or_else(|| "bridge".to_string());
    let restart = payload.restart.unwrap_or_else(|| "no".to_string());

    let _ = pull_image(&payload.image).await;

    match container_create(
        &name,
        &payload.image,
        cmd,
        env,
        "",
        volumes,
        ports,
        network == "host",
        &restart,
        0,
        0.0,
        None,
        false,
        &network,
    ) {
        Ok(_) => {
            let _ = container_start(&name);
            (StatusCode::CREATED, Json(json!({ "status": "running", "id": name }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_start_container(Path(id): Path<String>) -> Response {
    match container_start(&id) {
        Ok(_) => Json(json!({ "status": "started", "id": id })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_stop_container(Path(id): Path<String>) -> Response {
    match container_stop(&id) {
        Ok(_) => Json(json!({ "status": "stopped", "id": id })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_delete_container(Path(id): Path<String>) -> Response {
    let _ = container_stop(&id);
    match container_delete(&id) {
        Ok(_) => Json(json!({ "status": "deleted", "id": id })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_get_logs(Path(id): Path<String>) -> Response {
    match container_logs(&id) {
        Ok(logs) => Json(json!({ "id": id, "logs": logs })).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_compose_up(Json(payload): Json<ComposePayload>) -> Response {
    match compose_up(&payload.file_path) {
        Ok(out) => Json(json!({ "output": out })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_compose_down(Json(payload): Json<ComposePayload>) -> Response {
    match compose_down(&payload.file_path) {
        Ok(out) => Json(json!({ "output": out })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_list_images() -> Json<serde_json::Value> {
    let images = list_images().unwrap_or_default();
    Json(json!({ "images": images }))
}

#[derive(Deserialize)]
pub struct NativePullPayload {
    pub image: String,
}

async fn native_pull_image(Json(payload): Json<NativePullPayload>) -> Response {
    match pull_image(&payload.image).await {
        Ok(_) => Json(json!({ "status": "success", "image": payload.image })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_list_volumes() -> Json<serde_json::Value> {
    let volumes = list_volumes();
    Json(json!({ "volumes": volumes }))
}

async fn native_list_networks() -> Json<serde_json::Value> {
    let networks = list_networks();
    Json(json!({ "networks": networks }))
}

async fn native_get_stats(Path(id): Path<String>) -> Response {
    match crate::container::read_container_stats(&id) {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "error": e }))).into_response(),
    }
}

async fn native_terminal_ws(
    ws: axum::extract::ws::WebSocketUpgrade,
    Path(id): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| {
        crate::daemon::docker_api::handle_pty_session(socket, id, vec!["/bin/sh".to_string()])
    })
}

