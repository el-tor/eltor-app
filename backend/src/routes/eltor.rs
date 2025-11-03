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
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use std::io::SeekFrom;

use crate::eltor::activate_eltord_process;
use crate::state::{AppState, EltordStatusResponse, MessageResponse};

#[axum::debug_handler(state = AppState)]
pub async fn activate_eltord_route(
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    
    let mode_clone = mode.clone();
    
    // **Important** Spawn on blocking thread pool to completely isolate from tokio runtime
    // This prevents any async code in activate_eltord_process from interfering
    tokio::task::spawn_blocking(move || {
        activate_eltord_process(mode_clone);
    });

    ResponseJson(MessageResponse {
        message: format!("{} activation started", mode),
    })
}

#[axum::debug_handler(state = AppState)]
pub async fn deactivate_eltord_route(
    AxumState(_state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    
    // Directly await the async deactivate function
    match crate::eltor::deactivate_eltord_process(mode).await {
        Ok(message) => ResponseJson(MessageResponse { message }),
        Err(e) => ResponseJson(MessageResponse { message: e }),
    }
}

// Status endpoint
pub async fn get_eltord_status(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<EltordStatusResponse> {
    // Use PID file checking instead of manager
    let status = crate::eltor::get_eltord_status_from_pid_files(&state.path_config).await;
    
    ResponseJson(EltordStatusResponse {
        running: status.running,
        client_running: status.client_running,
        relay_running: status.relay_running,
        pid: None,
        recent_logs: vec![],
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

/// Stream eltord logs from file via SSE (tail -f behavior)
pub async fn stream_eltord_logs(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let path_config = state.path_config.clone();
    
    let stream = async_stream::stream! {
        // Determine log file based on mode
        let log_file = if let Some(app_data_dir) = &path_config.app_data_dir {
            // Tauri mode - logs are in app data directory
            if mode == "relay" {
                app_data_dir.join("eltor-relay.log")
            } else {
                app_data_dir.join("eltor.log")
            }
        } else {
            // Non-Tauri mode - logs are in bin/data
            if mode == "relay" {
                path_config.bin_dir.join("data").join("eltor-relay.log")
            } else {
                path_config.bin_dir.join("data").join("eltor.log")
            }
        };
        
        log::info!("📡 Starting log stream for: {:?}", log_file);
        
        // Wait for file to exist (up to 10 seconds)
        let mut attempts = 0;
        while !log_file.exists() && attempts < 20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            attempts += 1;
        }
        
        if !log_file.exists() {
            log::error!("❌ Log file does not exist: {:?}", log_file);
            return;
        }
        
        let mut file = match File::open(&log_file).await {
            Ok(f) => f,
            Err(e) => {
                log::error!("❌ Failed to open log file: {}", e);
                return;
            }
        };
        
        // Seek to end of file to only get new lines (tail -f behavior)
        if let Err(e) = file.seek(SeekFrom::End(0)).await {
            log::error!("❌ Failed to seek log file: {}", e);
            return;
        }
        
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        
        loop {
            line.clear();
            
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - wait and retry (tail -f behavior)
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Ok(_) => {
                    // Send the line as SSE event
                    let event = Event::default()
                        .data(line.trim())
                        .event("log");
                    yield Ok(event);
                }
                Err(e) => {
                    log::error!("❌ Error reading log line: {}", e);
                    break;
                }
            }
        }
    };
    
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Get recent logs from file (for initial load)
#[derive(serde::Serialize)]
pub struct LogsResponse {
    pub logs: Vec<String>,
}

pub async fn get_eltord_logs(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<LogsResponse> {
     let log_file = if let Some(app_data_dir) = &state.path_config.app_data_dir {
        if mode == "relay" {
            app_data_dir.join("eltor-relay.log")
        } else {
            app_data_dir.join("eltor.log")
        }
    } else {
        if mode == "relay" {
            state.path_config.bin_dir.join("data").join("eltor-relay.log")
        } else {
            state.path_config.bin_dir.join("data").join("eltor.log")
        }
    };
    
    let logs = match tokio::fs::read_to_string(&log_file).await {
        Ok(content) => {
            content
                .lines()
                .rev()
                .take(100)
                .map(|s| s.to_string())
                .collect()
        }
        Err(_) => vec![],
    };
    
    ResponseJson(LogsResponse { logs })
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
        .route("/api/eltord/logs/:mode", get(get_eltord_logs))           // GET recent logs
        .route("/api/eltord/logs/stream/:mode", get(stream_eltord_logs)) // SSE stream
}
