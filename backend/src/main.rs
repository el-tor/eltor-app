use axum::{
    extract::State as AxumState,
    http::{HeaderValue, Method, HeaderName},
    response::{Json as ResponseJson, Sse, sse::Event},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use serde::Serialize;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::CorsLayer;

// Log entry structure
#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: String,
    message: String,
    source: String, // "stdout" or "stderr"
}

// Shared state for tracking eltord process and logs
#[derive(Clone)]
struct AppState {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    log_sender: broadcast::Sender<LogEntry>,
    recent_logs: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl AppState {
    fn new() -> Self {
        let (log_sender, _) = broadcast::channel(1000);
        Self {
            process: Arc::new(Mutex::new(None)),
            log_sender,
            recent_logs: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
        }
    }

    fn add_log(&self, entry: LogEntry) {
        // Add to recent logs with rotation
        {
            let mut logs = self.recent_logs.lock().unwrap();
            if logs.len() >= 100 {
                logs.pop_front();
            }
            logs.push_back(entry.clone());
        }

        // Send to broadcast channel (ignore errors if no receivers)
        let _ = self.log_sender.send(entry);
    }

    fn get_recent_logs(&self) -> Vec<LogEntry> {
        self.recent_logs.lock().unwrap().clone().into()
    }
}

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    circuit: Option<String>,
}

#[derive(Serialize)]
struct EltordStatusResponse {
    running: bool,
    pid: Option<u32>,
    recent_logs: Vec<LogEntry>,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn connect_tor() -> ResponseJson<StatusResponse> {
    println!("🔗 Connecting to Tor...");
    // TODO: Implement actual Tor connection logic
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

async fn disconnect_tor() -> ResponseJson<StatusResponse> {
    println!("🔌 Disconnecting from Tor...");
    // TODO: Implement actual Tor disconnection logic
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

async fn get_tor_status() -> ResponseJson<StatusResponse> {
    // TODO: Implement actual Tor status check
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

// Function to read logs from a process stream
async fn read_process_logs(
    mut reader: AsyncBufReader<tokio::process::ChildStdout>,
    state: AppState,
    source: &'static str,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let entry = LogEntry {
                        timestamp: Utc::now(),
                        level: "INFO".to_string(), // Could parse this from the log line
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading {} logs: {}", source, e);
                break;
            }
        }
    }
}

// Function to read stderr logs
async fn read_process_stderr_logs(
    mut reader: AsyncBufReader<tokio::process::ChildStderr>,
    state: AppState,
    source: &'static str,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let entry = LogEntry {
                        timestamp: Utc::now(),
                        level: "ERROR".to_string(), // stderr usually contains errors
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading {} logs: {}", source, e);
                break;
            }
        }
    }
}

#[axum::debug_handler(state = AppState)]
async fn activate_eltord(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<MessageResponse> {
    // Check if already running
    {
        let mut process_guard = state.process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited, clear it
                    *process_guard = None;
                }
                Ok(None) => {
                    // Process is still running, but continue anyway
                    println!("⚠️ Eltord process already running, continuing...");
                }
                Err(_) => {
                    // Error checking process, assume it's dead
                    *process_guard = None;
                }
            }
        }
    }

    let eltord_path = match dirs::home_dir() {
        Some(home) => home.join("code/eltord"),
        None => {
            return ResponseJson(MessageResponse {
                message: "Error: Could not find home directory".to_string(),
            });
        }
    };
    
    println!("🚀 Running eltord from: {:?}", eltord_path);
    
    let mut child = match TokioCommand::new("cargo")
        .arg("run")
        .arg("--")
        .arg("client")
        .arg("-f")
        .arg("torrc.client.prod")
        .arg("-pw")
        .arg("password1234_")
        .current_dir(&eltord_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return ResponseJson(MessageResponse {
                message: format!("Error: Failed to start eltord: {}", e),
            });
        }
    };
    
    let pid = child.id().unwrap_or(0);
    
    // Set up log readers
    if let Some(stdout) = child.stdout.take() {
        let reader = AsyncBufReader::new(stdout);
        let state_clone = state.clone();
        tokio::spawn(async move {
            read_process_logs(reader, state_clone, "stdout").await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = state.clone();
        tokio::spawn(async move {
            read_process_stderr_logs(reader, state_clone, "stderr").await;
        });
    }
    
    // Store the process
    {
        let mut process_guard = state.process.lock().unwrap();
        *process_guard = Some(child);
    }
    
    println!("✅ Eltord process started with PID: {}", pid);
    
    // Add startup log
    state.add_log(LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: format!("Eltord process started with PID: {}", pid),
        source: "system".to_string(),
    });
    
    ResponseJson(MessageResponse {
        message: format!("Eltord activated with PID: {}", pid),
    })
}

#[axum::debug_handler(state = AppState)]
async fn deactivate_eltord(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<MessageResponse> {
    // Take the process out of the mutex first to avoid holding the lock across await
    let mut child = {
        let mut process_guard = state.process.lock().unwrap();
        process_guard.take()
    };
    
    if let Some(ref mut child) = child {
        match child.kill().await {
            Ok(_) => {
                println!("🛑 Killing eltord process...");
                let _ = child.wait().await; // Wait for process to actually terminate
                println!("✅ Eltord process terminated successfully");
                
                // Add shutdown log
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: "Eltord process terminated".to_string(),
                    source: "system".to_string(),
                });
                
                ResponseJson(MessageResponse {
                    message: "Eltord deactivated successfully".to_string(),
                })
            }
            Err(e) => {
                ResponseJson(MessageResponse {
                    message: format!("Error: Failed to kill eltord process: {}", e),
                })
            }
        }
    } else {
        ResponseJson(MessageResponse {
            message: "Error: No eltord process is currently running".to_string(),
        })
    }
}

async fn get_eltord_status(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<EltordStatusResponse> {
    let mut process_guard = state.process.lock().unwrap();
    
    let (running, pid) = if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited
                *process_guard = None;
                (false, None)
            }
            Ok(None) => {
                // Process is still running
                (true, child.id())
            }
            Err(_) => {
                // Error checking process, assume it's dead
                *process_guard = None;
                (false, None)
            }
        }
    } else {
        (false, None)
    };

    let recent_logs = state.get_recent_logs();

    ResponseJson(EltordStatusResponse { 
        running, 
        pid,
        recent_logs,
    })
}

// SSE endpoint for streaming logs
async fn stream_logs(
    AxumState(state): AxumState<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.log_sender.subscribe();
    let stream = BroadcastStream::new(receiver)
        .map(|result| {
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

async fn health_check() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Backend server is running".to_string(),
    })
}

#[tokio::main]
async fn main() {
    // Initialize shared state
    let state = AppState::new();

    // Configure CORS to allow SSE
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("cache-control"),
        ])
        .allow_credentials(true);

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/tor/connect", post(connect_tor))
        .route("/api/tor/disconnect", post(disconnect_tor))
        .route("/api/tor/status", get(get_tor_status))
        .route("/api/eltord/activate", post(activate_eltord))
        .route("/api/eltord/deactivate", post(deactivate_eltord))
        .route("/api/eltord/status", get(get_eltord_status))
        .route("/api/eltord/logs", get(stream_logs))
        .layer(cors)
        .with_state(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("🚀 El Tor Backend Server");
    println!("📡 Running on http://localhost:8080");
    println!("🔗 Health check: http://localhost:8080/health");
    println!("📋 API endpoints:");
    println!("   POST /api/tor/connect");
    println!("   POST /api/tor/disconnect");
    println!("   GET  /api/tor/status");
    println!("   POST /api/eltord/activate");
    println!("   POST /api/eltord/deactivate");
    println!("   GET  /api/eltord/status");
    println!("   GET  /api/eltord/logs (SSE)");
    
    axum::serve(listener, app).await.unwrap();
}
