use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use chrono::Utc;

use crate::state::{AppState, LogEntry};

// Function to read phoenixd logs from stdout
async fn read_phoenixd_logs(
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
                        level: "INFO".to_string(),
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[phoenixd-{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading phoenixd {} logs: {}", source, e);
                break;
            }
        }
    }
}

// Function to read phoenixd logs from stderr
async fn read_phoenixd_stderr_logs(
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
                        level: "WARN".to_string(), // phoenixd stderr might be warnings
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[phoenixd-{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                println!("Error reading phoenixd {} logs: {}", source, e);
                break;
            }
        }
    }
}

pub async fn start_phoenixd(state: AppState) -> Result<(), String> {
    // Check if phoenixd is already running
    {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited, clear it
                    *process_guard = None;
                }
                Ok(None) => {
                    // Process is still running
                    println!("⚠️ Phoenixd process already running, skipping startup");
                    return Ok(());
                }
                Err(_) => {
                    // Error checking process, assume it's dead
                    *process_guard = None;
                }
            }
        }
    }

    // Get the path to the phoenixd binary
    let current_dir = std::env::current_dir().map_err(|e| {
        format!("Error getting current directory: {}", e)
    })?;
    
    // Check multiple possible locations for the phoenixd binary
    let possible_paths = vec![
        current_dir.join("bin").join("phoenixd"),                    // Backend standalone
        current_dir.join("..").join("..").join("backend").join("bin").join("phoenixd"), // Tauri context
        current_dir.join("..").join("backend").join("bin").join("phoenixd"), // Alternative Tauri path
    ];
    
    let mut phoenixd_binary = None;
    for path in possible_paths {
        if path.exists() {
            phoenixd_binary = Some(path);
            break;
        }
    }
    
    let phoenixd_binary = phoenixd_binary.ok_or_else(|| {
        format!("Phoenixd binary not found. Searched paths: {:?}", 
            vec![
                current_dir.join("bin").join("phoenixd"),
                current_dir.join("..").join("..").join("backend").join("bin").join("phoenixd"),
                current_dir.join("..").join("backend").join("bin").join("phoenixd"),
            ]
        )
    })?;
    
    println!("🔥 Starting phoenixd from: {:?}", phoenixd_binary);
    
    let mut child = TokioCommand::new(&phoenixd_binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start phoenixd: {}", e))?;
    
    let pid = child.id().unwrap_or(0);
    
    // Set up log readers for phoenixd
    if let Some(stdout) = child.stdout.take() {
        let reader = AsyncBufReader::new(stdout);
        let state_clone = state.clone();
        tokio::spawn(async move {
            read_phoenixd_logs(reader, state_clone, "phoenixd-stdout").await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = state.clone();
        tokio::spawn(async move {
            read_phoenixd_stderr_logs(reader, state_clone, "phoenixd-stderr").await;
        });
    }
    
    // Store the phoenixd process
    {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        *process_guard = Some(child);
    }
    
    println!("✅ Phoenixd process started with PID: {}", pid);
    
    // Add startup log
    state.add_log(LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: format!("Phoenixd wallet process started with PID: {}", pid),
        source: "system".to_string(),
    });
    
    Ok(())
}

pub async fn stop_phoenixd(state: AppState) -> Result<(), String> {
    // Take the process out of the mutex first to avoid holding the lock across await
    let mut child = {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        process_guard.take()
    };
    
    if let Some(ref mut child) = child {
        match child.kill().await {
            Ok(_) => {
                println!("🛑 Killing phoenixd process...");
                let _ = child.wait().await; // Wait for process to actually terminate
                println!("✅ Phoenixd process terminated successfully");
                
                // Add shutdown log
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: "Phoenixd wallet process terminated".to_string(),
                    source: "system".to_string(),
                });
                
                Ok(())
            }
            Err(e) => {
                Err(format!("Failed to kill phoenixd process: {}", e))
            }
        }
    } else {
        Err("No phoenixd process is currently running".to_string())
    }
}
