use axum::{
    extract::State as AxumState,
    response::{Json as ResponseJson, Sse, sse::Event},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use futures::stream::Stream;
use std::convert::Infallible;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::state::{AppState, LogEntry, EltordStatusResponse, MessageResponse};

// Function to read logs from a process stream
async fn read_process_logs(
    mut reader: AsyncBufReader<tokio::process::ChildStdout>,
    state: AppState,
    source: &'static str,
    mode: String,
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
                        mode: Some(mode.clone()),
                    };
                    
                    println!("[{}-{}] {}", mode, source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading {}-{} logs: {}", mode, source, e);
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
    mode: String,
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
                        mode: Some(mode.clone()),
                    };
                    
                    println!("[{}-{}] {}", mode, source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading {}-{} logs: {}", mode, source, e);
                break;
            }
        }
    }
}

pub fn get_bin_dir() -> std::path::PathBuf {
    // Get the path to the eltord binary from ./bin folder (relative to backend crate)
    // Use the CARGO_MANIFEST_DIR environment variable to find the backend directory
    let backend_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(|manifest_dir| {
            let path = std::path::Path::new(&manifest_dir);
            // If CARGO_MANIFEST_DIR points to src-tauri, navigate to backend
            if path.ends_with("src-tauri") {
                path.parent().unwrap().parent().unwrap().join("backend").to_string_lossy().to_string()
            } else if path.file_name().unwrap() == "backend" {
                // Already in backend directory
                manifest_dir
            } else {
                // Try to find backend directory from manifest location
                path.join("backend").to_string_lossy().to_string()
            }
        })
        .unwrap_or_else(|_| {
            // Fallback: try to find the backend directory relative to current dir
            let current_dir = std::env::current_dir().unwrap();
            println!("🔍 Current working directory: {:?}", current_dir);
            
            // If we're in src-tauri, go up to find backend
            if current_dir.ends_with("src-tauri") {
                current_dir.parent().unwrap().parent().unwrap().join("backend").to_string_lossy().to_string()
            } else if current_dir.ends_with("frontend") {
                current_dir.parent().unwrap().join("backend").to_string_lossy().to_string()
            } else {
                // Assume we're in the root project directory
                current_dir.join("backend").to_string_lossy().to_string()
            }
        });
    
    let bin_dir: std::path::PathBuf = std::path::Path::new(&backend_dir).join("bin");

    let eltord_binary = bin_dir.join("eltord");
    if !eltord_binary.exists() {
        println!("🔍 eltord not found in bin directory, checking current directory...");
        let current_dir = std::env::current_dir().unwrap();
        let current_eltord = current_dir.join("eltord");
        if current_eltord.exists() {
            println!("✅ Found eltord in current directory: {:?}", current_dir);
            return current_dir;
        }
    }

    bin_dir
}

#[axum::debug_handler(state = AppState)]
pub async fn activate_eltord(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<MessageResponse> {
    // Check if already running (using client process for legacy compatibility)
    {
        let mut process_guard = state.client_process.lock().unwrap();
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

    let bin_dir = get_bin_dir();
    let eltord_binary = bin_dir.join("eltord");
    let torrc_file = bin_dir.join("data").join("torrc");
    
    println!("🔍 Looking for eltord binary at: {:?}", eltord_binary);
    println!("🔍 Looking for torrc file at: {:?}", torrc_file);
    
    // Check if the eltord binary exists
    if !eltord_binary.exists() {
        return ResponseJson(MessageResponse {
            message: format!("Error: eltord binary not found at {:?}", eltord_binary),
        });
    }
    
    // Check if the torrc file exists
    if !torrc_file.exists() {
        return ResponseJson(MessageResponse {
            message: format!("Error: torrc file not found at {:?}", torrc_file),
        });
    }
    
    println!("🚀 Running eltord binary from: {:?}", eltord_binary);
    println!("📋 Using torrc file: {:?}", torrc_file);
    
    let mut child = match TokioCommand::new(&eltord_binary)
        .arg("client")
        .arg("-f")
        .arg(&torrc_file)
        .arg("-pw")
        .arg("password1234_")
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
            read_process_logs(reader, state_clone, "stdout", "client".to_string()).await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = state.clone();
        tokio::spawn(async move {
            read_process_stderr_logs(reader, state_clone, "stderr", "client".to_string()).await;
        });
    }
    
    // Store the process (using client process for legacy compatibility)
    {
        let mut process_guard = state.client_process.lock().unwrap();
        *process_guard = Some(child);
    }
    
    println!("✅ Eltord process started with PID: {}", pid);
    
    // Add startup log
    state.add_log(LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: format!("Eltord process started with PID: {}", pid),
        source: "system".to_string(),
        mode: Some("client".to_string()),
    });
    
    ResponseJson(MessageResponse {
        message: format!("Eltord activated with PID: {}", pid),
    })
}

// Mode-aware activation function - supports both client and relay modes
pub async fn activate_eltord_internal(
    state: AppState,
    mode: String,
    torrc_file_name: Option<String>,
) -> ResponseJson<MessageResponse> {
    // Validate mode parameter first
    if mode != "client" && mode != "relay" {
        return ResponseJson(MessageResponse {
            message: format!("Error: Invalid mode '{}'. Must be 'client' or 'relay'", mode),
        });
    }

    // Check if already running for this specific mode
    {
        let process_mutex = if mode == "client" {
            &state.client_process
        } else {
            &state.relay_process
        };

        let mut process_guard = process_mutex.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited, clear it
                    *process_guard = None;
                }
                Ok(None) => {
                    // Process is still running for this mode
                    println!("⚠️ Eltord {} process already running, continuing...", mode);
                }
                Err(_) => {
                    // Error checking process, assume it's dead
                    *process_guard = None;
                }
            }
        }
    }

    let bin_dir = get_bin_dir();
    let eltord_binary = bin_dir.join("eltord");
    
    // Use provided torrc_file_name or default to state's torrc_file_name
    let torrc_file_name = torrc_file_name.unwrap_or_else(|| state.torrc_file_name.clone());
    let torrc_file = bin_dir.join("data").join(&torrc_file_name);
    
    println!("🔍 Looking for eltord binary at: {:?}", eltord_binary);
    println!("🔍 Looking for torrc file at: {:?}", torrc_file);
    
    // Check if the eltord binary exists
    if !eltord_binary.exists() {
        return ResponseJson(MessageResponse {
            message: format!("Error: eltord binary not found at {:?}", eltord_binary),
        });
    }
    
    // Check if the torrc file exists
    if !torrc_file.exists() {
        return ResponseJson(MessageResponse {
            message: format!("Error: torrc file not found at {:?}", torrc_file),
        });
    }
    
    println!("🚀 Running eltord binary from: {:?}", eltord_binary);
    println!("📋 Using torrc file: {:?}", torrc_file);
    
    // Clean up any Tor processes using ports from this specific torrc file (preserve phoenixd)
    println!("🧹 Cleaning up Tor ports for torrc: {} (preserving phoenixd)", torrc_file_name);
    if let Err(e) = crate::ports::cleanup_tor_ports_only(&torrc_file_name).await {
        println!("⚠️  Tor port cleanup failed: {}", e);
        println!("   Continuing with eltord startup...");
    }
    
    // Validate mode parameter
    if mode != "client" && mode != "relay" {
        return ResponseJson(MessageResponse {
            message: format!("Error: Invalid mode '{}'. Must be 'client' or 'relay'", mode),
        });
    }

    let mut child = match TokioCommand::new(&eltord_binary)
        .arg(&mode)
        .arg("-f")
        .arg(&torrc_file)
        .arg("-pw")
        .arg("password1234_")
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
    
    // Set up log readers with mode information
    if let Some(stdout) = child.stdout.take() {
        let reader = AsyncBufReader::new(stdout);
        let state_clone = state.clone();
        let mode_clone = mode.clone();
        tokio::spawn(async move {
            read_process_logs(reader, state_clone, "stdout", mode_clone).await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = state.clone();
        let mode_clone = mode.clone();
        tokio::spawn(async move {
            read_process_stderr_logs(reader, state_clone, "stderr", mode_clone).await;
        });
    }
    
    // Store the process in the appropriate field based on mode
    {
        let process_mutex = if mode == "client" {
            &state.client_process
        } else {
            &state.relay_process
        };

        let mut process_guard = process_mutex.lock().unwrap();
        *process_guard = Some(child);
    }
    
    println!("✅ Eltord process started with PID: {} using mode: {} and torrc: {}", pid, mode, torrc_file_name);
    
    // Add startup log
    state.add_log(LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: format!("Eltord process started with PID: {} using mode: {} and torrc: {}", pid, mode, torrc_file_name),
        source: "system".to_string(),
        mode: Some(mode.clone()),
    });
    
    ResponseJson(MessageResponse {
        message: format!("Eltord activated with PID: {} using mode: {} and torrc: {}", pid, mode, torrc_file_name),
    })
}

#[axum::debug_handler(state = AppState)]
pub async fn deactivate_eltord(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<MessageResponse> {
    // Take the process out of the mutex first to avoid holding the lock across await (legacy client compatibility)
    let mut child = {
        let mut process_guard = state.client_process.lock().unwrap();
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
                    mode: Some("client".to_string()),
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

pub async fn get_eltord_status(
    AxumState(state): AxumState<AppState>,
) -> ResponseJson<EltordStatusResponse> {
    let mut process_guard = state.client_process.lock().unwrap();
    
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

// Mode-aware deactivation function
pub async fn deactivate_eltord_internal(
    state: AppState,
    mode: Option<String>,
) -> ResponseJson<MessageResponse> {
    let mode = mode.unwrap_or_else(|| "client".to_string());
    
    // Validate mode parameter
    if mode != "client" && mode != "relay" {
        return ResponseJson(MessageResponse {
            message: format!("Error: Invalid mode '{}'. Must be 'client' or 'relay'", mode),
        });
    }

    // Take the process out of the mutex first to avoid holding the lock across await
    let mut child = {
        let process_mutex = if mode == "client" {
            &state.client_process
        } else {
            &state.relay_process
        };

        let mut process_guard = process_mutex.lock().unwrap();
        process_guard.take()
    };
    
    if let Some(ref mut child) = child {
        match child.kill().await {
            Ok(_) => {
                println!("🛑 Killing eltord {} process...", mode);
                let _ = child.wait().await; // Wait for process to actually terminate
                println!("✅ Eltord {} process terminated successfully", mode);
                
                // Add shutdown log
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: format!("Eltord {} process terminated", mode),
                    source: "system".to_string(),
                    mode: Some(mode.clone()),
                });
                
                ResponseJson(MessageResponse {
                    message: format!("Eltord {} deactivated successfully", mode),
                })
            }
            Err(e) => {
                ResponseJson(MessageResponse {
                    message: format!("Error: Failed to kill eltord {} process: {}", mode, e),
                })
            }
        }
    } else {
        ResponseJson(MessageResponse {
            message: format!("Error: No eltord {} process is currently running", mode),
        })
    }
}

// Mode-aware activation endpoint
#[axum::debug_handler(state = AppState)]
pub async fn activate_eltord_with_mode(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    activate_eltord_internal(state, mode, None).await
}

// Mode-aware activation endpoint with torrc file
#[axum::debug_handler(state = AppState)]
pub async fn activate_eltord_with_mode_and_torrc(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path((mode, torrc_file_name)): axum::extract::Path<(String, String)>,
) -> ResponseJson<MessageResponse> {
    activate_eltord_internal(state, mode, Some(torrc_file_name)).await
}

// Mode-aware deactivation endpoint
#[axum::debug_handler(state = AppState)]
pub async fn deactivate_eltord_with_mode(
    AxumState(state): AxumState<AppState>,
    axum::extract::Path(mode): axum::extract::Path<String>,
) -> ResponseJson<MessageResponse> {
    deactivate_eltord_internal(state, Some(mode)).await
}

// SSE endpoint for streaming logs
pub async fn stream_logs(
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

// Create eltord routes
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/eltord/activate", post(activate_eltord))
        .route("/api/eltord/activate/:mode", post(activate_eltord_with_mode))
        .route("/api/eltord/activate/:mode/:torrc_file_name", post(activate_eltord_with_mode_and_torrc))
        .route("/api/eltord/deactivate", post(deactivate_eltord))
        .route("/api/eltord/deactivate/:mode", post(deactivate_eltord_with_mode))
        .route("/api/eltord/status", get(get_eltord_status))
        .route("/api/eltord/logs", get(stream_logs))
}