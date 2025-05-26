use axum::{
    extract::Json,
    http::{HeaderValue, Method},
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

// Shared state for tracking eltord process
type SharedState = Arc<Mutex<Option<Child>>>;

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    circuit: Option<String>,
}

#[derive(Serialize)]
struct EltordStatusResponse {
    running: bool,
    pid: Option<u32>,
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

async fn activate_eltord(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> Result<ResponseJson<MessageResponse>, axum::response::Json<ErrorResponse>> {
    let mut process_guard = state.lock().unwrap();
    
    // Check if already running
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited, clear it
                *process_guard = None;
            }
            Ok(None) => {
                // Process is still running
                return Err(ResponseJson(ErrorResponse {
                    error: "Eltord is already running".to_string(),
                }));
            }
            Err(_) => {
                // Error checking process, assume it's dead
                *process_guard = None;
            }
        }
    }

    let eltord_path = dirs::home_dir()
        .ok_or_else(|| ResponseJson(ErrorResponse {
            error: "Could not find home directory".to_string(),
        }))?
        .join("code/eltord");
    
    println!("🚀 Running eltord from: {:?}", eltord_path);
    
    let child = Command::new("cargo")
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
        .map_err(|e| ResponseJson(ErrorResponse {
            error: format!("Failed to start eltord: {}", e),
        }))?;
    
    let pid = child.id();
    *process_guard = Some(child);
    
    println!("✅ Eltord process started with PID: {}", pid);
    
    Ok(ResponseJson(MessageResponse {
        message: format!("Eltord activated with PID: {}", pid),
    }))
}

async fn deactivate_eltord(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> Result<ResponseJson<MessageResponse>, axum::response::Json<ErrorResponse>> {
    let mut process_guard = state.lock().unwrap();
    
    if let Some(mut child) = process_guard.take() {
        match child.kill() {
            Ok(_) => {
                println!("🛑 Killing eltord process...");
                let _ = child.wait(); // Wait for process to actually terminate
                println!("✅ Eltord process terminated successfully");
                
                Ok(ResponseJson(MessageResponse {
                    message: "Eltord deactivated successfully".to_string(),
                }))
            }
            Err(e) => {
                Err(ResponseJson(ErrorResponse {
                    error: format!("Failed to kill eltord process: {}", e),
                }))
            }
        }
    } else {
        Err(ResponseJson(ErrorResponse {
            error: "No eltord process is currently running".to_string(),
        }))
    }
}

async fn get_eltord_status(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> ResponseJson<EltordStatusResponse> {
    let mut process_guard = state.lock().unwrap();
    
    let (running, pid) = if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited
                *process_guard = None;
                (false, None)
            }
            Ok(None) => {
                // Process is still running
                (true, Some(child.id()))
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

    ResponseJson(EltordStatusResponse { running, pid })
}

async fn health_check() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Backend server is running".to_string(),
    })
}

#[tokio::main]
async fn main() {
    // Initialize shared state for tracking eltord process
    let state = Arc::new(Mutex::new(None));

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/tor/connect", post(connect_tor))
        .route("/api/tor/disconnect", post(disconnect_tor))
        .route("/api/tor/status", get(get_tor_status))
        .route("/api/eltord/activate", post(activate_eltord))
        .route("/api/eltord/deactivate", post(deactivate_eltord))
        .route("/api/eltord/status", get(get_eltord_status))
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
    
    axum::serve(listener, app).await.unwrap();
}