// Library entrypoint for eltor-backend
// This allows the backend to be imported as a library by Tauri

use std::sync::Arc;
use tokio::sync::RwLock;
use log::{Log, Metadata, Record};
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
    EltorManager, EltorStatus,
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

use crate::eltor::EltorMode;

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
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // Capture all log levels
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Try to determine the mode from the log content or source
            let mode = self.determine_log_mode(record);
            
            let log_entry = LogEntry {
                timestamp: Utc::now(),
                level: record.level().to_string(),
                message: format!("{}", record.args()),
                source: record.target().to_string(),
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
pub fn create_app_state(use_phoenixd_embedded: bool) -> AppState {
    AppState::new(use_phoenixd_embedded)
}

/// Set up custom logger to capture ALL logs (including from eltor library) and send them to broadcast channel
/// This should be called early in the application startup, before any other logging occurs
pub fn setup_broadcast_logger(state: AppState) -> Result<(), String> {
    let broadcast_logger = BroadcastLogger::new(state);
    
    // Initialize the custom logger FIRST to capture all subsequent log output
    if let Err(e) = log::set_boxed_logger(Box::new(broadcast_logger)) {
        return Err(format!("Failed to set custom logger: {}", e));
    }
    
    log::set_max_level(log::LevelFilter::Trace); // Capture all log levels
    println!("🎯 Custom logger installed successfully - ALL logs will stream to broadcast channel");
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
    let manager = eltor::EltorManager::new(state.clone(), path_config);
    app_state.set_eltor_manager(manager);
    Ok(())
}

/// Initialize phoenixd if embedded mode is enabled - must be called from async context
pub async fn initialize_phoenixd(state: Arc<RwLock<AppState>>) -> Result<(), String> {
    let app_state = state.read().await;
    if app_state.wallet_state.use_phoenixd_embedded {
        println!("🔥 Initializing embedded phoenixd...");
        let cloned_state = AppState {
            client_task: app_state.client_task.clone(),
            relay_task: app_state.relay_task.clone(),
            log_sender: app_state.log_sender.clone(),
            recent_logs: app_state.recent_logs.clone(),
            wallet_state: app_state.wallet_state.clone(),
            lightning_node: app_state.lightning_node.clone(),
            torrc_file_name: app_state.torrc_file_name.clone(),
            eltor_manager: app_state.eltor_manager.clone(),
        };
        drop(app_state);
        wallet::start_phoenixd(cloned_state).await
    } else {
        println!("🔗 Using external phoenixd instance");
        Ok(())
    }
}

/// Activate eltord - requires manager in AppState
pub async fn activate_eltord(state: Arc<RwLock<AppState>>, mode: EltorMode) -> Result<String, String> {
    let app_state = state.read().await;
    let manager = match &app_state.eltor_manager {
        Some(manager) => manager.clone(),
        None => return Err("EltorManager not initialized in AppState".to_string()),
    };
    drop(app_state); // Release the read lock
    
    let params = EltorActivateParams { mode };
    manager.activate(params).await
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

/// Get eltord status - requires manager in AppState
pub async fn get_eltord_status(state: Arc<RwLock<AppState>>) -> EltordStatusResponse {
    let app_state = state.read().await;
    let manager = match &app_state.eltor_manager {
        Some(manager) => manager.clone(),
        None => {
            return EltordStatusResponse {
                running: false,
                client_running: false,
                relay_running: false,
                pid: None,
                recent_logs: vec![],
            }
        }
    };
    drop(app_state); // Release the read lock
    
    let status = manager.get_status().await;
    EltordStatusResponse {
        running: status.running,
        client_running: status.client_running,
        relay_running: status.relay_running,
        pid: None,
        recent_logs: status.recent_logs,
    }
}

/// Get a log receiver for listening to log events
pub async fn get_log_receiver(state: Arc<RwLock<AppState>>) -> broadcast::Receiver<LogEntry> {
    let app_state = state.read().await;
    app_state.log_sender.subscribe()
}

/// Comprehensive shutdown function - cleans up all processes and ports
pub async fn shutdown_cleanup(state: Arc<RwLock<AppState>>) -> Result<(), String> {
    println!("🛑 Starting comprehensive app shutdown cleanup...");

    match deactivate_eltord(state.clone(), EltorMode::Relay).await {
        Ok(msg) => println!("✅ Relay: {}", msg),
        Err(e) => {
            if e.contains("No relay eltord process") || e.contains("not running") {
                println!("ℹ️  No relay eltord process to stop");
            } else {
                println!("⚠️  Relay eltord cleanup warning: {}", e);
            }
        }
    }

    // Step 2: Stop phoenixd process if running
    let app_state = state.read().await;
    let phoenixd_state = app_state.clone(); // Clone the AppState for phoenixd functions
    drop(app_state);
    
    match stop_phoenixd(phoenixd_state.clone()).await {
        Ok(_) => println!("✅ Phoenixd stopped successfully"),
        Err(e) => {
            if e.contains("No phoenixd process") {
                println!("ℹ️  No phoenixd process to stop");
            } else {
                println!("⚠️  Phoenixd cleanup warning: {}", e);
            }
        }
    }

    // Step 3: Clean up all ports (including phoenixd)
    println!("🧹 Cleaning up all application ports...");
    match cleanup_ports_with_torrc(&phoenixd_state.torrc_file_name).await {
        Ok(_) => println!("✅ All ports cleaned up successfully"),
        Err(e) => println!("⚠️  Port cleanup warning: {}", e),
    }

    println!("✨ App shutdown cleanup completed");
    Ok(())
}
