use crate::cronet::CronetEngine;
use crate::cronet_pb::{ExecuteRequest, ExecuteResponse};
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use std::sync::Arc;

// Service State
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<CronetEngine>,
}

// Handlers
pub async fn execute_request(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> impl IntoResponse {
    eprintln!(
        "[DEBUG] execute_request handler entered. Request ID: {}",
        request.request_id
    );
    // Validate Target
    let target = match request.target {
        Some(t) => t,
        None => {
            return Json(ExecuteResponse {
                request_id: request.request_id,
                success: false,
                error_message: "Missing target configuration".to_string(),
                ..Default::default()
            })
        }
    };

    // Start Timer
    let start_time = std::time::Instant::now();

    // Execute Request via Cronet
    // Note: We currently only support URL and Method. Headers/Body support pending.
    let config_default = crate::cronet_pb::ExecutionConfig::default();
    let config = request.config.as_ref().unwrap_or(&config_default);

    let (request_handle, rx) = state.engine.start_request(&target, config);

    // Wait for result
    let execution_result = rx.await;
    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Drop the request handle after we are done
    drop(request_handle);

    match execution_result {
        Ok(Ok(res)) => {
            // Success
            Json(ExecuteResponse {
                request_id: request.request_id,
                success: true,
                error_message: "".to_string(),
                duration_ms,
                response: Some(crate::cronet_pb::TargetResponse {
                    status_code: res.status_code,
                    headers: Default::default(), // TODO: Maps headers
                    body: res.body,
                }),
            })
        }
        Ok(Err(err_msg)) => {
            // Cronet Error (Failed/Canceled)
            Json(ExecuteResponse {
                request_id: request.request_id,
                success: false,
                error_message: err_msg,
                duration_ms,
                response: None,
            })
        }
        Err(_) => {
            // RecvError (Internal Panic)
            Json(ExecuteResponse {
                request_id: request.request_id,
                success: false,
                error_message: "Internal Executor Error".to_string(),
                duration_ms,
                response: None,
            })
        }
    }
}

#[derive(serde::Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub service: String,
}

pub async fn get_version(State(_state): State<AppState>) -> Json<VersionResponse> {
    let version = env!("CRONET_VERSION").to_string();
    Json(VersionResponse {
        version,
        service: "cronet-cloak".to_string(),
    })
}
