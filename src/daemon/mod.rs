pub mod docker_api;
pub mod native_api;
pub mod zl_engine;
pub mod slots;

use axum::Router;
use std::os::unix::fs::PermissionsExt;
use tower_http::cors::{Any, CorsLayer};
use zl_engine::ZlScriptLoader;

pub async fn run_daemon(port: u16, socket_path: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(docker_api::docker_router())
        .merge(native_api::native_router())
        .layer(cors);

    let mode_str = if ZlScriptLoader::is_dev_mode() {
        "Dev Mode (Hot-Reload zsrc/)"
    } else {
        "Release Mode (Embedded .zl in Memory)"
    };

    let addr = format!("0.0.0.0:{}", port);
    let target_socket = socket_path.unwrap_or_else(|| "/var/run/docker.sock".to_string());

    println!("⚡ [zenobox daemon] Netva REST API & Docker Socket API server listening on http://{}", addr);
    println!("   └─ ZenoLang Engine: {}", mode_str);
    println!("   └─ Standard REST API: http://{}/api/v1/containers", addr);
    println!("   └─ Docker API Compatibility: http://{}/v1.41/_ping", addr);

    // Try binding Unix domain socket for 1Panel and Docker CLI compatibility
    let _ = std::fs::remove_file(&target_socket);
    if let Ok(unix_listener) = tokio::net::UnixListener::bind(&target_socket) {
        let _ = std::fs::set_permissions(&target_socket, std::fs::Permissions::from_mode(0o666));
        println!("   └─ Unix Socket API: unix://{}", target_socket);
        let app_unix = app.clone();
        tokio::spawn(async move {
            let _ = axum::serve(unix_listener, app_unix).await;
        });
    } else {
        println!("   └─ Unix Socket API: (Could not bind to {})", target_socket);
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
