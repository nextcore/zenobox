pub mod docker_api;
pub mod native_api;
pub mod zl_engine;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use zl_engine::ZlScriptLoader;

pub async fn run_daemon(port: u16, _socket_path: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
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
    println!("⚡ [zenobox daemon] Netva REST API & Docker Socket API server listening on http://{}", addr);
    println!("   └─ ZenoLang Engine: {}", mode_str);
    println!("   └─ Standard REST API: http://{}/api/v1/containers", addr);
    println!("   └─ Docker API Compatibility: http://{}/v1.41/_ping", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
