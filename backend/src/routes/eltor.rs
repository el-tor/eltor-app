use axum::{
    extract::State as AxumState,
    response::{sse::Event, Json as ResponseJson, Sse},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::eltor::{EltorActivateParams, EltorDeactivateParams, EltorMode};
use crate::state::{AppState, EltordStatusResponse, MessageResponse};

#[axum::debug_handler(state = AppState)]
pub async fn activate_eltord_route(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    let manager = match &state.eltor_manager {
        Some(manager) => manager.clone(),
        None => {
            return ResponseJson(MessageResponse {
                message: "EltorManager not initialized".to_string(),
            })
        }
    };

    let params = EltorActivateParams {
        mode: EltorMode::from_str(&mode).unwrap_or(EltorMode::Client),
    };

    match manager.activate(params).await {
        Ok(message) => ResponseJson(MessageResponse { message }),
        Err(e) => ResponseJson(MessageResponse { message: e }),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn deactivate_eltord_route(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    let manager = match &state.eltor_manager {
        Some(manager) => manager.clone(),
        None => {
            return ResponseJson(MessageResponse {
                message: "EltorManager not initialized".to_string(),
            });
        }
    };

    let params = EltorDeactivateParams {
        mode: EltorMode::from_str(&mode).unwrap_or(EltorMode::Client),
    };

    match manager.deactivate(params).await {
        Ok(message) => ResponseJson(MessageResponse { message }),
        Err(e) => ResponseJson(MessageResponse { message: e }),
    }
}

// Status endpoint
pub async fn get_eltord_status(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<EltordStatusResponse> {
    let manager = match &state.eltor_manager {
        Some(manager) => manager.clone(),
        None => {
            return ResponseJson(EltordStatusResponse {
                running: false,
                client_running: false,
                relay_running: false,
                pid: None,
                recent_logs: vec![],
            })
        }
    };

    let status = manager.get_status().await;
    ResponseJson(EltordStatusResponse {
        running: status.running,
        client_running: status.client_running,
        relay_running: status.relay_running,
        pid: None,
        recent_logs: status.recent_logs,
    })
}

// SSE endpoint for streaming logs
pub async fn stream_logs(
    AxumState(state): AxumState<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.log_sender.subscribe();
    let stream = BroadcastStream::new(receiver).map(|result| {
        match result {
            Ok(log_entry) => {
                let json = serde_json::to_string(&log_entry).unwrap_or_default();
                Ok(Event::default().data(json))
            }
            Err(_) => {
                // Channel lagged, send error event
                Ok(Event::default().data("{\"error\":\"stream_lagged\"}"))
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

// Create eltord routes
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/eltord/activate/:mode", post(activate_eltord_route))
        .route(
            "/api/eltord/deactivate/:mode",
            post(deactivate_eltord_route),
        )
        .route("/api/eltord/status", get(get_eltord_status))
        .route("/api/eltord/logs", get(stream_logs))
}
