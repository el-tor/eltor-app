use chrono::Utc;
use log::info;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;

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

/// Reap exited children; a retained handle alone is not running status.
pub fn phoenixd_pid(state: &AppState) -> Result<Option<u32>, String> {
    let mut process = state.wallet_state.phoenixd_process.lock().unwrap();
    if let Some(child) = process.as_mut() {
        match child.try_wait() {
            Ok(None) => return Ok(child.id()),
            Ok(Some(_)) => *process = None,
            Err(_) => return Err("Unable to inspect managed Phoenix process".into()),
        }
    }
    Ok(None)
}
pub async fn start_phoenixd(state: AppState) -> Result<(), String> {
    let _lifecycle = state.wallet_state.phoenixd_lifecycle.lock().await;
    spawn_phoenixd(&state)
}
/// Caller holds the shared lifecycle lock through readiness/configuration.
pub(crate) fn spawn_phoenixd(state: &AppState) -> Result<(), String> {
    if phoenixd_pid(state)?.is_some() {
        return Ok(());
    }
    let path_config = &state.path_config;
    let phoenixd_binary = path_config.get_executable_path("phoenixd");

    if !phoenixd_binary.exists() {
        return Err(format!(
            "Phoenixd binary not found at: {:?}",
            phoenixd_binary
        ));
    }

    info!("🔥 Starting phoenixd from: {:?}", phoenixd_binary);

    // Set phoenixd working directory to app data directory to ensure it can write files
    let phoenixd_working_dir = path_config.data_dir.join("phoenixd");
    std::fs::create_dir_all(&phoenixd_working_dir)
        .map_err(|_| "Unable to create Phoenix working directory".to_string())?;

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
        let state_clone = (*state).clone();
        tokio::spawn(async move {
            read_phoenixd_logs(reader, state_clone, "phoenixd-stdout").await;
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = (*state).clone();
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
    let _lifecycle = state.wallet_state.phoenixd_lifecycle.lock().await;
    // Take the process out of the mutex first to avoid holding the lock across await
    let child = {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        process_guard.take()
    };
    if let Some(mut child) = child {
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
                *state.wallet_state.phoenixd_process.lock().unwrap() = Some(child);
                Err(format!("Failed to kill phoenixd process: {}", e))
            }
        }
    } else {
        Err("No phoenixd process is currently running".to_string())
    }
}

/// Auto-start is limited to the user's existing selected local wallet.
pub(crate) fn phoenix_autostart_allowed(
    enabled: bool,
    credentials: bool,
    node_type: &str,
    url: &str,
    is_default: bool,
) -> bool {
    enabled
        && credentials
        && is_default
        && node_type == "phoenixd"
        && reqwest::Url::parse(url).ok().map_or(false, |url| {
            url.scheme() == "http"
                && url.port_or_known_default() == Some(9740)
                && matches!(
                    url.host_str(),
                    Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
                )
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn autostart_requires_existing_selected_local_wallet_and_explicit_enablement() {
        assert!(phoenix_autostart_allowed(
            true,
            true,
            "phoenixd",
            "http://127.0.0.1:9740",
            true
        ));
        assert!(!phoenix_autostart_allowed(
            false,
            true,
            "phoenixd",
            "http://127.0.0.1:9740",
            true
        ));
        assert!(!phoenix_autostart_allowed(
            true,
            false,
            "phoenixd",
            "http://127.0.0.1:9740",
            true
        ));
        assert!(!phoenix_autostart_allowed(
            true,
            true,
            "phoenixd",
            "https://wallet.example",
            true
        ));
        assert!(!phoenix_autostart_allowed(
            true,
            true,
            "cln",
            "http://127.0.0.1:9740",
            true
        ));
        assert!(!phoenix_autostart_allowed(
            true,
            true,
            "phoenixd",
            "http://127.0.0.1:9740",
            false
        ));
    }
}
