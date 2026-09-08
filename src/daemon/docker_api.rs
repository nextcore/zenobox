use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query,
    },
    http::{header::HeaderName, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};

use crate::container::{
    container_create, container_delete, container_exec, container_list_internal, container_logs,
    container_start, container_stop, read_container_stats,
};
use crate::image::{list_images, pull_image};
use crate::utils::{get_data_dir, get_runc_bin};

#[derive(Deserialize)]
pub struct ContainerListQuery {
    pub all: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateContainerPayload {
    pub image: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    #[serde(rename = "Env")]
    pub env: Option<Vec<String>>,
    #[serde(rename = "HostConfig")]
    pub host_config: Option<DockerHostConfig>,
}

#[derive(Deserialize)]
pub struct DockerHostConfig {
    #[serde(rename = "PortBindings")]
    pub port_bindings: Option<HashMap<String, Vec<HashMap<String, String>>>>,
    #[serde(rename = "Binds")]
    pub binds: Option<Vec<String>>,
    #[serde(rename = "NetworkMode")]
    pub network_mode: Option<String>,
}

#[derive(Deserialize)]
pub struct ExecCreatePayload {
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    #[serde(rename = "AttachStdin")]
    pub attach_stdin: Option<bool>,
    #[serde(rename = "AttachStdout")]
    pub attach_stdout: Option<bool>,
    #[serde(rename = "AttachStderr")]
    pub attach_stderr: Option<bool>,
    #[serde(rename = "Tty")]
    pub tty: Option<bool>,
}

#[derive(Deserialize)]
pub struct ExecStartPayload {
    #[serde(rename = "Detach")]
    pub detach: Option<bool>,
    #[serde(rename = "Tty")]
    pub tty: Option<bool>,
}

pub fn docker_router() -> Router {
    let mut router = Router::new()
        .route("/_ping", get(ping))
        .route("/info", get(docker_info))
        .route("/containers/json", get(list_containers_docker))
        .route("/containers/create", post(create_container_docker))
        .route("/containers/{id}/start", post(start_container_docker))
        .route("/containers/{id}/stop", post(stop_container_docker))
        .route("/containers/{id}", delete(delete_container_docker))
        .route("/containers/{id}/logs", get(get_container_logs_docker))
        .route("/containers/{id}/stats", get(get_container_stats_docker))
        .route("/containers/{id}/attach/ws", get(attach_ws_docker))
        .route("/containers/{id}/exec", post(create_exec_docker))
        .route("/exec/{id}/start", post(start_exec_docker))
        .route("/exec/{id}/ws", get(exec_ws_docker))
        .route("/exec/{id}/json", get(inspect_exec_docker))
        .route("/images/json", get(list_images_docker))
        .route("/images/create", post(pull_image_docker));

    let versions = ["v1.40", "v1.41", "v1.42", "v1.43", "v1.44", "v1.45", "v1.46", "v1.47"];
    for v in versions {
        router = router
            .route(&format!("/{}/_ping", v), get(ping))
            .route(&format!("/{}/info", v), get(docker_info))
            .route(&format!("/{}/containers/json", v), get(list_containers_docker))
            .route(&format!("/{}/containers/create", v), post(create_container_docker))
            .route(&format!("/{}/containers/{{id}}/start", v), post(start_container_docker))
            .route(&format!("/{}/containers/{{id}}/stop", v), post(stop_container_docker))
            .route(&format!("/{}/containers/{{id}}", v), delete(delete_container_docker))
            .route(&format!("/{}/containers/{{id}}/logs", v), get(get_container_logs_docker))
            .route(&format!("/{}/containers/{{id}}/stats", v), get(get_container_stats_docker))
            .route(&format!("/{}/containers/{{id}}/attach/ws", v), get(attach_ws_docker))
            .route(&format!("/{}/containers/{{id}}/exec", v), post(create_exec_docker))
            .route(&format!("/{}/exec/{{id}}/start", v), post(start_exec_docker))
            .route(&format!("/{}/exec/{{id}}/ws", v), get(exec_ws_docker))
            .route(&format!("/{}/exec/{{id}}/json", v), get(inspect_exec_docker))
            .route(&format!("/{}/images/json", v), get(list_images_docker))
            .route(&format!("/{}/images/create", v), post(pull_image_docker));
    }

    router
}

async fn ping() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("api-version"), "1.41".parse().unwrap());
    headers.insert(HeaderName::from_static("docker-experimental"), "false".parse().unwrap());
    headers.insert(HeaderName::from_static("builder-version"), "zenobox/0.1.0".parse().unwrap());
    (StatusCode::OK, headers, "OK").into_response()
}

async fn docker_info() -> Json<serde_json::Value> {
    Json(json!({
        "ID": "ZENOBOX-DAEMON-01",
        "Containers": 0,
        "ContainersRunning": 0,
        "ContainersPaused": 0,
        "ContainersStopped": 0,
        "Images": 0,
        "Driver": "overlay2",
        "OperatingSystem": "Linux (Zenobox)",
        "NCPU": 4,
        "MemTotal": 8589934592u64,
        "ServerVersion": "zenobox/0.1.0"
    }))
}

async fn list_containers_docker(Query(q): Query<ContainerListQuery>) -> Json<serde_json::Value> {
    let data_dir = get_data_dir();
    let show_all = q.all.unwrap_or(false);
    let containers = container_list_internal(&data_dir, true).unwrap_or_default();

    let mut result = Vec::new();
    for c in containers {
        if !show_all && c.status != "running" {
            continue;
        }

        let state_str = if c.status == "running" { "running" } else { "exited" };
        let status_str = format!("Up ({})", c.status);

        let mut ports_json = Vec::new();
        if let Some(ref p_list) = c.ports {
            for p in p_list {
                let parts: Vec<&str> = p.split(':').collect();
                if parts.len() == 2 {
                    ports_json.push(json!({
                        "PublicPort": parts[0].parse::<u16>().unwrap_or(0),
                        "PrivatePort": parts[1].parse::<u16>().unwrap_or(0),
                        "Type": "tcp"
                    }));
                }
            }
        }

        result.push(json!({
            "Id": c.id,
            "Names": [format!("/{}", c.id)],
            "Image": c.image,
            "State": state_str,
            "Status": status_str,
            "Created": 1600000000,
            "Ports": ports_json
        }));
    }

    Json(json!(result))
}

async fn create_container_docker(
    Query(params): Query<HashMap<String, String>>,
    Json(payload): Json<CreateContainerPayload>,
) -> Response {
    let name_query = params.get("name").cloned();
    let container_name = payload.name.or(name_query).unwrap_or_else(|| format!("zeno-{}", rand::random::<u32>()));
    let image = payload.image.unwrap_or_else(|| "alpine:latest".to_string());
    let cmd = payload.cmd.unwrap_or_default();

    let mut env_map = HashMap::new();
    if let Some(env_list) = payload.env {
        for e in env_list {
            let parts: Vec<&str> = e.splitn(2, '=').collect();
            if parts.len() == 2 {
                env_map.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }

    let mut ports = Vec::new();
    let mut mounts = Vec::new();
    let mut net_mode = "bridge".to_string();

    if let Some(ref hc) = payload.host_config {
        if let Some(ref net) = hc.network_mode {
            net_mode = net.clone();
        }
        if let Some(ref binds) = hc.binds {
            mounts = binds.clone();
        }
        if let Some(ref pb) = hc.port_bindings {
            for (container_p, host_list) in pb {
                let clean_p = container_p.split('/').next().unwrap_or(container_p);
                for h in host_list {
                    if let Some(host_p) = h.get("HostPort") {
                        ports.push(format!("{}:{}", host_p, clean_p));
                    }
                }
            }
        }
    }

    let _ = pull_image(&image).await;

    match container_create(
        &container_name,
        &image,
        cmd,
        env_map,
        "",
        mounts,
        ports,
        net_mode == "host",
        "no",
        0,
        0.0,
        None,
        false,
        &net_mode,
    ) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "Id": container_name,
                "Warnings": []
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn start_container_docker(Path(id): Path<String>) -> Response {
    match container_start(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn stop_container_docker(Path(id): Path<String>) -> Response {
    match container_stop(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn delete_container_docker(Path(id): Path<String>) -> Response {
    let _ = container_stop(&id);
    match container_delete(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn get_container_logs_docker(Path(id): Path<String>) -> Response {
    match container_logs(&id) {
        Ok(logs) => (StatusCode::OK, logs).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn list_images_docker() -> Json<serde_json::Value> {
    let images = list_images().unwrap_or_default();
    let mut result = Vec::new();
    for img in images {
        result.push(json!({
            "Id": format!("sha256:{}", hex::encode(img.as_bytes())),
            "RepoTags": [img],
            "Created": 1600000000,
            "Size": 15000000u64
        }));
    }
    Json(json!(result))
}

#[derive(Deserialize)]
pub struct PullImageQuery {
    pub from_image: Option<String>,
    pub tag: Option<String>,
}

async fn pull_image_docker(Query(q): Query<PullImageQuery>) -> Response {
    let img_name = q.from_image.unwrap_or_else(|| "alpine".to_string());
    let tag = q.tag.unwrap_or_else(|| "latest".to_string());
    let full_ref = format!("{}:{}", img_name, tag);

    match pull_image(&full_ref).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "Download complete" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn create_exec_docker(
    Path(id): Path<String>,
    Json(payload): Json<ExecCreatePayload>,
) -> Response {
    let cmd = payload.cmd.unwrap_or_else(|| vec!["/bin/sh".to_string()]);
    let cmd_joined = cmd.join(" ");
    let exec_id = format!("{}:{}", id, hex::encode(&cmd_joined));

    (
        StatusCode::CREATED,
        Json(json!({
            "Id": exec_id
        })),
    )
        .into_response()
}

async fn start_exec_docker(
    Path(exec_id): Path<String>,
    _json: Option<Json<ExecStartPayload>>,
) -> Response {
    let parts: Vec<&str> = exec_id.splitn(2, ':').collect();
    let (container_id, cmd_str) = if parts.len() == 2 {
        let decoded = hex::decode(parts[1]).unwrap_or_default();
        let cmd = String::from_utf8(decoded).unwrap_or_else(|_| "/bin/sh".to_string());
        (parts[0], cmd)
    } else {
        (exec_id.as_str(), "/bin/sh".to_string())
    };

    let cmd_parts: Vec<&str> = cmd_str.split_whitespace().collect();
    let cmd_slice = if cmd_parts.is_empty() { vec!["/bin/sh"] } else { cmd_parts };

    match container_exec(container_id, &cmd_slice) {
        Ok(out) => (StatusCode::OK, out).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": e })),
        )
            .into_response(),
    }
}

async fn inspect_exec_docker(Path(_exec_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "CanRemove": true,
        "DetachKeys": "",
        "ExitCode": 0,
        "ID": _exec_id,
        "Running": false,
        "OpenStdin": false,
        "OpenStderr": false,
        "OpenStdout": true,
        "ContainerID": _exec_id.split(':').next().unwrap_or(&_exec_id)
    }))
}

#[derive(Deserialize)]
pub struct StatsQuery {
    pub stream: Option<bool>,
}

async fn get_container_stats_docker(
    Path(id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Response {
    let should_stream = q.stream.unwrap_or(true);
    if !should_stream {
        match read_container_stats(&id) {
            Ok(stats) => Json(stats).into_response(),
            Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "message": e }))).into_response(),
        }
    } else {
        let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            std::time::Duration::from_secs(1),
        ))
        .map(move |_| {
            match read_container_stats(&id) {
                Ok(stats) => Ok::<_, std::convert::Infallible>(format!("{}\n", stats.to_string())),
                Err(e) => Ok::<_, std::convert::Infallible>(format!("{{\"error\":\"{}\"}}\n", e)),
            }
        });
        Response::builder()
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from_stream(stream))
            .unwrap()
    }
}

async fn attach_ws_docker(ws: WebSocketUpgrade, Path(id): Path<String>) -> Response {
    ws.on_upgrade(move |socket| handle_pty_session(socket, id, vec!["/bin/sh".to_string()]))
}

async fn exec_ws_docker(ws: WebSocketUpgrade, Path(exec_id): Path<String>) -> Response {
    let parts: Vec<&str> = exec_id.splitn(2, ':').collect();
    let (container_id, cmd_str) = if parts.len() == 2 {
        let decoded = hex::decode(parts[1]).unwrap_or_default();
        let cmd = String::from_utf8(decoded).unwrap_or_else(|_| "/bin/sh".to_string());
        (parts[0].to_string(), cmd)
    } else {
        (exec_id.clone(), "/bin/sh".to_string())
    };

    let cmd_parts: Vec<String> = cmd_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let cmd_vec = if cmd_parts.is_empty() {
        vec!["/bin/sh".to_string()]
    } else {
        cmd_parts
    };

    ws.on_upgrade(move |socket| handle_pty_session(socket, container_id, cmd_vec))
}

pub async fn handle_pty_session(socket: WebSocket, container_id: String, cmd: Vec<String>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => {
            let _ = ws_sender
                .send(Message::Text(format!("Failed to open PTY: {}\r\n", e).into()))
                .await;
            return;
        }
    };

    let runc_bin = get_runc_bin();
    let mut cmd_builder = CommandBuilder::new(runc_bin);
    cmd_builder.arg("exec");
    cmd_builder.arg("-t");
    cmd_builder.arg(&container_id);
    for c in &cmd {
        cmd_builder.arg(c);
    }

    let mut child = match pair.slave.spawn_command(cmd_builder) {
        Ok(c) => c,
        Err(e) => {
            let _ = ws_sender
                .send(Message::Text(format!("Failed to exec process: {}\r\n", e).into()))
                .await;
            return;
        }
    };

    drop(pair.slave);

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            let _ = ws_sender
                .send(Message::Text(format!("Failed to clone master reader: {}\r\n", e).into()))
                .await;
            return;
        }
    };

    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            let _ = ws_sender
                .send(Message::Text(format!("Failed to take master writer: {}\r\n", e).into()))
                .await;
            return;
        }
    };

    let (tx_out, mut rx_out) = tokio::sync::mpsc::channel::<Vec<u8>>(100);

    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx_out.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let master_pty = pair.master;

    let send_task = tokio::spawn(async move {
        while let Some(bytes) = rx_out.recv().await {
            if ws_sender.send(Message::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(txt) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                        if let (Some(rows), Some(cols)) = (
                            val.get("rows").and_then(|r| r.as_u64()),
                            val.get("cols").and_then(|c| c.as_u64()),
                        ) {
                            let _ = master_pty.resize(PtySize {
                                rows: rows as u16,
                                cols: cols as u16,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                            continue;
                        }
                    }
                    let _ = writer.write_all(txt.as_bytes());
                    let _ = writer.flush();
                }
                Message::Binary(bin) => {
                    let _ = writer.write_all(&bin);
                    let _ = writer.flush();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let _ = tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    };

    let _ = child.kill();
}

