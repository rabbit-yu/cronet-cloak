use axum::{routing::post, Router};
use cronet_cloak::cronet;
use cronet_cloak::service;
use cronet_cloak::service::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize Cronet Engine
    let engine = Arc::new(cronet::CronetEngine::new("CronetCloak/1.0"));
    let state = AppState { engine };

    // Build Router
    // Connect-style path: /<package>.<Service>/<Method>
    // Package: cronet.engine.v1
    // Service: EngineService
    // Method: Execute
    let app = Router::new()
        // Connect-RPC compatible path
        .route(
            "/cronet.engine.v1.EngineService/Execute",
            post(service::execute_request),
        )
        // Simple REST path alias
        .route("/api/execute", post(service::execute_request))
        .route("/api/v1/execute", post(service::execute_request))
        // Version endpoint
        .route("/version", axum::routing::get(service::get_version))
        .route("/api/version", axum::routing::get(service::get_version))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
