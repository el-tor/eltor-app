// Library entrypoint for eltor-backend
// This allows the backend to be imported as a library by Tauri

use std::sync::Arc;
use tokio::sync::RwLock;

pub mod eltor;
pub mod lightning;
pub mod paths;
pub mod ports;
pub mod routes;
pub mod state;
pub mod static_files;
pub mod torrc_parser;
pub mod wallet;

// Re-export commonly used types for convenience
pub use eltor::{
    activate_eltor, deactivate_eltor, get_eltor_status, EltorActivateParams, EltorDeactivateParams,
    EltorManager, EltorStatus,
};
pub use lightning::{LightningNode, ListTransactionsResponse, WalletBalanceResponse};
pub use paths::PathConfig;
pub use ports::{
    cleanup_ports, cleanup_ports_startup, cleanup_ports_with_torrc, cleanup_tor_ports_only,
    get_ports_to_check, get_tor_ports_only,
};
pub use state::{AppState, EltordStatusResponse, LogEntry, MessageResponse, StatusResponse};
use tokio::sync::broadcast;
pub use wallet::{start_phoenixd, stop_phoenixd};

// Re-export IP location types and functions
pub use routes::ip::{init_ip_database, lookup_ip_location, IpLocationResponse};

use crate::eltor::EltorMode;

/// Create a new AppState for Tauri usage
pub fn create_app_state(use_phoenixd_embedded: bool) -> AppState {
    AppState::new(use_phoenixd_embedded)
}

/// Initialize phoenixd if embedded mode is enabled - must be called from async context
pub async fn initialize_phoenixd(state: AppState) -> Result<(), String> {
    if state.wallet_state.use_phoenixd_embedded {
        println!("🔥 Initializing embedded phoenixd...");
        wallet::start_phoenixd(state).await
    } else {
        println!("🔗 Using external phoenixd instance");
        Ok(())
    }
}

/// Activate eltord
pub async fn activate_eltord(state: AppState, mode: EltorMode) -> Result<String, String> {
    let state_arc = Arc::new(RwLock::new(state));
    let path_config = PathConfig::new()?;
    let manager = EltorManager::new(state_arc, path_config);
    let params = EltorActivateParams { mode };
    manager.activate(params).await
}

/// Deactivate eltord
pub async fn deactivate_eltord(state: AppState, mode: EltorMode) -> Result<String, String> {
    let state_arc = Arc::new(RwLock::new(state));
    let path_config = PathConfig::new()?;
    let manager = EltorManager::new(state_arc, path_config);
    let params = EltorDeactivateParams { mode };
    manager.deactivate(params).await
}

/// Get eltord status
pub async fn get_eltord_status(state: AppState) -> EltordStatusResponse {
    let state_arc = Arc::new(RwLock::new(state));
    let path_config = match PathConfig::new() {
        Ok(config) => config,
        Err(_) => {
            return EltordStatusResponse {
                running: false,
                client_running: false,
                relay_running: false,
                pid: None,
                recent_logs: vec![],
            }
        }
    };
    let manager = EltorManager::new(state_arc, path_config);
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
pub fn get_log_receiver(state: &AppState) -> broadcast::Receiver<LogEntry> {
    state.log_sender.subscribe()
}

/// Comprehensive shutdown function - cleans up all processes and ports
pub async fn shutdown_cleanup(state: AppState) -> Result<(), String> {
    println!("🛑 Starting comprehensive app shutdown cleanup...");

    match deactivate_eltord(state.clone(), EltorMode::Relay).await {
        Ok(msg) => println!("✅ Relay: {}", msg),
        Err(e) => {
            if e.contains("No relay eltord process") {
                println!("ℹ️  No relay eltord process to stop");
            } else {
                println!("⚠️  Relay eltord cleanup warning: {}", e);
            }
        }
    }

    // Step 2: Stop phoenixd process if running
    match stop_phoenixd(state.clone()).await {
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
    match cleanup_ports_with_torrc(&state.torrc_file_name).await {
        Ok(_) => println!("✅ All ports cleaned up successfully"),
        Err(e) => println!("⚠️  Port cleanup warning: {}", e),
    }

    println!("✨ App shutdown cleanup completed");
    Ok(())
}
