// Library entrypoint for eltor-backend
// This allows the backend to be imported as a library by Tauri

pub mod lightning;
pub mod ports;
pub mod routes;
pub mod state;
pub mod static_files;
pub mod torrc_parser;
pub mod wallet;

// Re-export commonly used types for convenience
pub use lightning::{LightningNode, WalletBalanceResponse, ListTransactionsResponse};
pub use state::{AppState, MessageResponse, StatusResponse, LogEntry, EltordStatusResponse};
pub use routes::eltor::{activate_eltord as backend_activate, deactivate_eltord as backend_deactivate, get_eltord_status as backend_status, get_bin_dir, activate_eltord_internal};
pub use ports::{get_ports_to_check, cleanup_ports, cleanup_ports_with_torrc, cleanup_ports_startup, cleanup_tor_ports_only, get_tor_ports_only};
pub use wallet::{start_phoenixd, stop_phoenixd};
use tokio::sync::broadcast;

// Re-export IP location types and functions
pub use routes::ip::{IpLocationResponse, lookup_ip_location, init_ip_database};

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

/// Activate eltord using backend logic - thin wrapper for Tauri
pub async fn activate_eltord_wrapper(state: AppState, torrc_file_name: Option<String>) -> Result<String, String> {
    use axum::response::Json as ResponseJson;
    
    let result = activate_eltord_internal(state, "client".to_string(), torrc_file_name).await;
    match result {
        ResponseJson(message_response) => {
            if message_response.message.starts_with("Error:") {
                Err(message_response.message)
            } else {
                Ok(message_response.message)
            }
        }
    }
}

/// Deactivate eltord using backend logic - thin wrapper for Tauri
pub async fn deactivate_eltord_wrapper(state: AppState) -> Result<String, String> {
    use axum::extract::State as AxumState;
    use axum::response::Json as ResponseJson;
    
    let result = backend_deactivate(AxumState(state)).await;
    match result {
        ResponseJson(message_response) => {
            if message_response.message.starts_with("Error:") {
                Err(message_response.message)
            } else {
                Ok(message_response.message)
            }
        }
    }
}

/// Deactivate eltord using backend logic with mode support - thin wrapper for Tauri
pub async fn deactivate_eltord_wrapper_with_mode(state: AppState, mode: String) -> Result<String, String> {
    use axum::response::Json as ResponseJson;
    
    let result = routes::eltor::deactivate_eltord_internal(state, Some(mode)).await;
    match result {
        ResponseJson(message_response) => {
            if message_response.message.starts_with("Error:") {
                Err(message_response.message)
            } else {
                Ok(message_response.message)
            }
        }
    }
}

/// Get eltord status using backend logic - thin wrapper for Tauri
pub async fn get_eltord_status_wrapper(state: AppState) -> EltordStatusResponse {
    use axum::extract::State as AxumState;
    use axum::response::Json as ResponseJson;
    
    let result = backend_status(AxumState(state)).await;
    match result {
        ResponseJson(status_response) => status_response,
    }
}

/// Activate eltord using backend logic with mode support - thin wrapper for Tauri
pub async fn activate_eltord_wrapper_with_mode(state: AppState, torrc_file_name: Option<String>, mode: String) -> Result<String, String> {
    use axum::response::Json as ResponseJson;
    
    let result = routes::eltor::activate_eltord_internal(state, mode, torrc_file_name).await;
    match result {
        ResponseJson(message_response) => {
            if message_response.message.starts_with("Error:") {
                Err(message_response.message)
            } else {
                Ok(message_response.message)
            }
        }
    }
}

/// Get a log receiver for listening to log events
pub fn get_log_receiver(state: &AppState) -> broadcast::Receiver<LogEntry> {
    state.log_sender.subscribe()
}

/// Comprehensive shutdown function - cleans up all processes and ports
pub async fn shutdown_cleanup(state: AppState) -> Result<(), String> {
    println!("🛑 Starting comprehensive app shutdown cleanup...");
    
    // Step 1: Stop both eltord processes if running
    match deactivate_eltord_wrapper_with_mode(state.clone(), "client".to_string()).await {
        Ok(msg) => println!("✅ Client: {}", msg),
        Err(e) => {
            if e.contains("No client eltord process") {
                println!("ℹ️  No client eltord process to stop");
            } else {
                println!("⚠️  Client eltord cleanup warning: {}", e);
            }
        }
    }
    
    match deactivate_eltord_wrapper_with_mode(state.clone(), "relay".to_string()).await {
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
