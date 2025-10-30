// Library entrypoint for eltor-backend
// This allows the backend to be imported as a library by Tauri

use std::sync::Arc;
use axum::extract::path;
use tokio::sync::RwLock;
use log::{Log, Metadata, Record, info};
use chrono::Utc;

pub mod eltor;
pub mod ip;
pub mod lightning;
pub mod paths;
pub mod ports;
pub mod routes;
pub mod state;
pub mod static_files;
pub mod torrc_parser;
pub mod wallet;
pub mod debug_info;

// Re-export commonly used types for convenience
pub use eltor::{
    EltorActivateParams, EltorDeactivateParams,
    EltorManager, EltorStatus, cleanup_all_eltord_processes,
};
pub use lightning::{LightningNode, ListTransactionsResponse, WalletBalanceResponse};
pub use paths::PathConfig;
pub use ports::{
    cleanup_ports, cleanup_ports_startup, cleanup_ports_with_torrc, cleanup_tor_ports_only,
    get_ports_to_check, get_tor_ports_only, cleanup_backend_port,
};
pub use state::{AppState, EltordStatusResponse, LogEntry, MessageResponse, StatusResponse};
use tokio::sync::broadcast;
pub use wallet::{start_phoenixd, stop_phoenixd, read_phoenixd_logs, read_phoenixd_stderr_logs};
pub use debug_info::DebugInfo;

// Re-export IP location types and functions
pub use routes::ip::{init_ip_database, lookup_ip_location, IpLocationResponse};

// Re-export Phoenix download functions
pub use routes::phoenix::{download_phoenix, download_phoenix_default, start_phoenix_with_config, PhoenixStartResponse};

use crate::eltor::{activate_eltord_process, EltorMode};

// Custom logger that captures all log messages and sends them to the broadcast channel
struct BroadcastLogger {
    state: AppState,
}

impl BroadcastLogger {
    fn new(state: AppState) -> Self {
        Self { state }
    }
    
    /// Determine the mode from the log record content and context
    fn determine_log_mode(&self, record: &Record) -> Option<String> {
        let message = format!("{}", record.args());
        let source = record.target();
        
        // Check the log message for mode indicators
        if message.contains("both mode") || message.contains("mode both") {
            return Some("both".to_string());
        }
        
        if message.contains("relay mode") || message.contains("mode relay") {
            return Some("relay".to_string());
        }
        
        if message.contains("client mode") || message.contains("mode client") {
            return Some("client".to_string());
        }
        
        // Check for relay-specific patterns
        if message.contains("relay") || message.contains("Relay") {
            return Some("relay".to_string());
        }
        
        // Check the source/target for mode indicators
        if source.contains("relay") {
            return Some("relay".to_string());
        }
        
        // For eltor library logs, try to determine from the context
        if source.starts_with("eltor") {
            // If it's an eltor library log, try to infer from the message content
            if message.contains("Starting eltor both") || message.contains("eltor both") {
                return Some("both".to_string());
            }
            if message.contains("Starting eltor relay") || message.contains("eltor relay") {
                return Some("relay".to_string());
            }
            if message.contains("Starting eltor client") || message.contains("eltor client") {
                return Some("client".to_string());
            }
        }
        
        // Default to None (will be handled by frontend)
        None
    }
}

impl Log for BroadcastLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Only capture Info, Warn, Error logs to prevent UI freeze from Debug/Trace floods
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            
            // Filter out excessive process output logs from eltord library
            // These come from the process manager's stdout/stderr monitoring
            // Now handled by the isolated process's own stdout/stderr readers
            if message.starts_with("[ELTORD-STDOUT]") 
                || message.starts_with("[ELTORD-STDERR]")
                || message.starts_with("[ELTORD-TRACE]")
                || message.starts_with("[eltord-")  // Filter isolated process logs
                || message.contains("eltord-stdout")
                || message.contains("eltord-stderr") {
                // Skip these verbose logs - they're handled by the process isolation layer
                return;
            }
            
            // Filter out excessively verbose eltor library debug output
            // that comes from the Tor binary's network operations
            let source = record.target();
            if source.starts_with("eltor::") && message.len() > 500 {
                // Skip very long messages from eltor internals (likely network dumps)
                return;
            }
            
            // Try to determine the mode from the log content or source
            let mode = self.determine_log_mode(record);
            
            let log_entry = LogEntry {
                timestamp: Utc::now(),
                level: record.level().to_string(),
                message,
                source: source.to_string(),
                mode,
            };
            self.state.add_log(log_entry);
        }
    }

    fn flush(&self) {
        // No-op for our use case
    }
}

/// Create a new AppState for Tauri usage
pub fn create_app_state(use_phoenixd_embedded: bool, path_config: PathConfig) -> AppState {
    AppState::new(use_phoenixd_embedded, path_config)
}

/// Set up custom logger to capture ALL logs (including from eltor library) and send them to broadcast channel
/// This should be called early in the application startup, before any other logging occurs
pub fn setup_broadcast_logger(state: AppState) -> Result<(), String> {
    // let broadcast_logger = BroadcastLogger::new(state);
    
    // // Initialize the custom logger FIRST to capture all subsequent log output
    // if let Err(e) = log::set_boxed_logger(Box::new(broadcast_logger)) {
    //     return Err(format!("Failed to set custom logger: {}", e));
    // }
    
    // // Set max level to Info to block Debug and Trace logs from eltord library
    // // This prevents excessive log flooding that causes UI freezes
    // log::set_max_level(log::LevelFilter::Info);
    // info!("🎯 Custom logger installed successfully - capturing Info, Warn, Error logs only");
    Ok(())
}

/// Initialize app state with EltorManager - for Tauri usage
pub async fn initialize_app_state(state: Arc<RwLock<AppState>>) -> Result<(), String> {
    let mut app_state = state.write().await;
    let path_config = PathConfig::new().map_err(|e| format!("Failed to create PathConfig: {}", e))?;
    let manager = eltor::EltorManager::new(state.clone(), path_config);
    app_state.set_eltor_manager(manager);
    Ok(())
}

/// Initialize app state with EltorManager using a custom PathConfig - for Tauri with resource directory
pub async fn initialize_app_state_with_path_config(
    state: Arc<RwLock<AppState>>, 
    path_config: PathConfig
) -> Result<(), String> {
    let mut app_state = state.write().await;
    
    // Update the path_config in AppState
    app_state.path_config = Arc::new(path_config.clone());
    
    let manager = eltor::EltorManager::new(state.clone(), path_config);
    app_state.set_eltor_manager(manager);
    Ok(())
}

/// Initialize phoenixd if embedded mode is enabled - must be called from async context
pub async fn initialize_phoenixd(state: Arc<RwLock<AppState>>) -> Result<(), String> {
    let app_state = state.read().await;
    if app_state.wallet_state.use_phoenixd_embedded {
        info!("🔥 Initializing embedded phoenixd...");
        let cloned_state = AppState {
            client_task: app_state.client_task.clone(),
            relay_task: app_state.relay_task.clone(),
            log_sender: app_state.log_sender.clone(),
            recent_logs: app_state.recent_logs.clone(),
            wallet_state: app_state.wallet_state.clone(),
            lightning_node: app_state.lightning_node.clone(),
            torrc_file_name: app_state.torrc_file_name.clone(),
            eltor_manager: app_state.eltor_manager.clone(),
            path_config: app_state.path_config.clone(),
            client_log_cancel: app_state.client_log_cancel.clone(),
            relay_log_cancel: app_state.relay_log_cancel.clone(),
        };
        drop(app_state);
        wallet::start_phoenixd(cloned_state).await
    } else {
        info!("🔗 Using external phoenixd instance");
        Ok(())
    }
}

/// Activate eltord - requires manager in AppState
pub fn activate_eltord(mode: String) -> Result<String, String> {
    // **Important** Use spawn_blocking to isolate the synchronous process spawning from the async runtime
    // or else the C tor binary maybe have networking issues and interruptions
    tokio::task::spawn_blocking(move || {
        eltor::activate_eltord_process(mode);
    });
    
    Ok("activation started".to_string())
}

/// Deactivate eltord - requires manager in AppState
pub async fn deactivate_eltord(state: Arc<RwLock<AppState>>, mode: EltorMode) -> Result<String, String> {
    let app_state = state.read().await;
    let manager = match &app_state.eltor_manager {
        Some(manager) => manager.clone(),
        None => return Err("EltorManager not initialized in AppState".to_string()),
    };
    drop(app_state); // Release the read lock
    
    let params = EltorDeactivateParams { mode };
    manager.deactivate(params).await
}

/// Get eltord status - uses PID files instead of manager
pub async fn get_eltord_status(state: Arc<RwLock<AppState>>) -> EltordStatusResponse {
    let app_state = state.read().await;
    let status = crate::eltor::get_eltord_status_from_pid_files(&app_state.path_config).await;
    drop(app_state); // Release the read lock
    
    EltordStatusResponse {
        running: status.running,
        client_running: status.client_running,
        relay_running: status.relay_running,
        pid: None,
        recent_logs: vec![],
    }
}

/// Get a log receiver for listening to log events
pub async fn get_log_receiver(state: Arc<RwLock<AppState>>) -> broadcast::Receiver<LogEntry> {
    let app_state = state.read().await;
    app_state.log_sender.subscribe()
}

/// Get recent eltord logs from file (Tauri command)
pub async fn get_eltord_logs(state: Arc<RwLock<AppState>>, mode: String) -> Result<Vec<String>, String> {
    let app_state = state.read().await;
    
    let log_file = if mode == "relay" {
        app_state.path_config.bin_dir.join("data").join("eltor-relay.log")
    } else {
        app_state.path_config.bin_dir.join("data").join("eltor.log")
    };
    
    match tokio::fs::read_to_string(&log_file).await {
        Ok(content) => {
            // Get last 100 lines
            let logs: Vec<String> = content
                .lines()
                .rev()
                .take(100)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            Ok(logs)
        }
        Err(_) => Ok(vec![]),
    }
}

/// Stream eltord logs from file (Tauri command) - emits events to frontend
/// Note: This function signature works for both lib.rs export and Tauri command wrapper
pub async fn stream_eltord_logs_internal(
    state: Arc<RwLock<AppState>>,
    mode: String,
    emit_fn: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<(), String> {
    use tokio_util::sync::CancellationToken;
    
    let app_state = state.read().await;
    let path_config = app_state.path_config.clone();
    
    // Create cancellation token and store it
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    
    if mode == "client" {
        let mut client_cancel = app_state.client_log_cancel.lock().unwrap();
        *client_cancel = Some(cancel_token_clone);
    } else {
        let mut relay_cancel = app_state.relay_log_cancel.lock().unwrap();
        *relay_cancel = Some(cancel_token_clone);
    }
    
    drop(app_state);
    
    // Spawn a task to tail the log file
    tokio::spawn(async move {
        use tokio::fs::File;
        use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
        use tokio::select;
        
        // Determine log file path - use app_data_dir if available (Tauri mode)
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
        
        log::info!("📡 [Tauri] Starting log stream for: {:?}", log_file);
        log::info!("📡 [Tauri] app_data_dir present: {}", path_config.app_data_dir.is_some());
        
        // Wait for file to exist
        let mut attempts = 0;
        while !log_file.exists() && attempts < 20 {
            select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                    attempts += 1;
                }
                _ = cancel_token.cancelled() => {
                    log::info!("⏸️ [Tauri] Log stream cancelled before file existed for mode: {}", mode);
                    return;
                }
            }
        }
        
        if !log_file.exists() {
            log::error!("❌ [Tauri] Log file does not exist: {:?}", log_file);
            return;
        }
        
        let mut file = match File::open(&log_file).await {
            Ok(f) => f,
            Err(e) => {
                log::error!("❌ [Tauri] Failed to open log file: {}", e);
                return;
            }
        };
        
        // Seek to end for tail behavior
        if let Err(e) = file.seek(std::io::SeekFrom::End(0)).await {
            log::error!("❌ [Tauri] Failed to seek log file: {}", e);
            return;
        }
        
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        
        loop {
            line.clear();
            
            select! {
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => {
                            // No data, sleep a bit
                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
                                _ = cancel_token.cancelled() => {
                                    log::info!("⏸️ [Tauri] Log stream cancelled for mode: {}", mode);
                                    break;
                                }
                            }
                        }
                        Ok(_) => {
                            // Emit event via callback
                            emit_fn(line.trim().to_string());
                        }
                        Err(e) => {
                            log::error!("❌ [Tauri] Error reading log: {}", e);
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    log::info!("⏸️ [Tauri] Log stream cancelled for mode: {}", mode);
                    break;
                }
            }
        }
        
        log::info!("🧹 [Tauri] Log stream ended for mode: {}", mode);
    });
    
    Ok(())
}

/// Comprehensive shutdown function - cleans up all processes and ports
pub async fn shutdown_cleanup(state: Arc<RwLock<AppState>>) -> Result<(), String> {
    info!("🛑 Starting comprehensive app shutdown cleanup...");

    // Step 1: Stop managed eltord processes (from EltorManager)
    match deactivate_eltord(state.clone(), EltorMode::Relay).await {
        Ok(msg) => info!("✅ Relay: {}", msg),
        Err(e) => {
            if e.contains("No relay eltord process") || e.contains("not running") {
                info!("ℹ️  No relay eltord process to stop");
            } else {
                info!("⚠️  Relay eltord cleanup warning: {}", e);
            }
        }
    }

    // Step 1b: Also cleanup any PID-file based eltord processes (failsafe)
    cleanup_all_eltord_processes();

    // Step 2: Stop phoenixd process if running
    let app_state = state.read().await;
    let phoenixd_state = app_state.clone(); // Clone the AppState for phoenixd functions
    drop(app_state);
    
    match stop_phoenixd(phoenixd_state.clone()).await {
        Ok(_) => info!("✅ Phoenixd stopped successfully"),
        Err(e) => {
            if e.contains("No phoenixd process") {
                info!("ℹ️  No phoenixd process to stop");
            } else {
                info!("⚠️  Phoenixd cleanup warning: {}", e);
            }
        }
    }

    // Step 3: Clean up all ports (including phoenixd)
    info!("🧹 Cleaning up all application ports...");
    match cleanup_ports_with_torrc(&phoenixd_state.torrc_file_name).await {
        Ok(_) => info!("✅ All ports cleaned up successfully"),
        Err(e) => info!("⚠️  Port cleanup warning: {}", e),
    }

    info!("✨ App shutdown cleanup completed");
    Ok(())
}
