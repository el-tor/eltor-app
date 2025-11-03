use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use chrono::Utc;
use log::info;

use crate::paths::PathConfig;
use crate::state::{AppState, LogEntry};

// Function to read phoenixd logs from stdout
pub async fn read_phoenixd_logs(
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
                        mode: None, // Wallet logs are system-wide
                    };
                    
                    info!("[phoenixd-{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                info!("Error reading phoenixd {} logs: {}", source, e);
                break;
            }
        }
    }
}

// Function to read phoenixd logs from stderr
pub async fn read_phoenixd_stderr_logs(
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
                        mode: None, // Wallet logs are system-wide
                    };
                    
                    info!("[phoenixd-{}] {}", source, trimmed);
                    state.add_log(entry);
                }
            }
            Err(e) => {
                info!("Error reading phoenixd {} logs: {}", source, e);
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
                    info!("⚠️ Phoenixd process already running, skipping startup");
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
    // Use PathConfig to find the phoenixd binary
    let path_config = PathConfig::new().map_err(|e| {
        format!("Error getting path configuration: {}", e)
    })?;
    
    let phoenixd_binary = path_config.get_executable_path("phoenixd");
    
    if !phoenixd_binary.exists() {
        return Err(format!("Phoenixd binary not found at: {:?}", phoenixd_binary));
    }
    
    info!("🔥 Starting phoenixd from: {:?}", phoenixd_binary);
    
    // Set phoenixd working directory to app data directory to ensure it can write files
    let phoenixd_working_dir = path_config.data_dir.join("phoenixd");
    if let Err(e) = std::fs::create_dir_all(&phoenixd_working_dir) {
        info!("⚠️ Warning: Could not create phoenixd directory {:?}: {}", phoenixd_working_dir, e);
        info!("   Phoenixd will use current directory for data files");
    }
    
    let mut child = TokioCommand::new(&phoenixd_binary)
        .current_dir(&phoenixd_working_dir)
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
    
    info!("✅ Phoenixd process started with PID: {}", pid);
    
    // Add startup log
    state.add_log(LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: format!("Phoenixd wallet process started with PID: {}", pid),
        source: "system".to_string(),
        mode: None, // Wallet logs are system-wide
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
                info!("🛑 Killing phoenixd process...");
                let _ = child.wait().await; // Wait for process to actually terminate
                info!("✅ Phoenixd process terminated successfully");
                
                // Add shutdown log
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: "Phoenixd wallet process terminated".to_string(),
                    source: "system".to_string(),
                    mode: None, // Wallet logs are system-wide
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
