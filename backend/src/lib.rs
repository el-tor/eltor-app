// Library entrypoint for eltor-backend
// This allows the backend to be imported as a library by Tauri

pub mod lightning;
pub mod ports;
pub mod routes;
pub mod state;
pub mod torrc_parser;
pub mod wallet;

// Re-export commonly used types for convenience
pub use lightning::{LightningNode, NodeInfoResponse, WalletBalanceResponse, ListTransactionsResponse};
pub use state::{AppState, MessageResponse, StatusResponse, LogEntry, EltordStatusResponse};
pub use routes::eltor::{activate_eltord as backend_activate, deactivate_eltord as backend_deactivate, get_eltord_status as backend_status};
use tokio::sync::broadcast;

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
pub async fn activate_eltord_wrapper(state: AppState) -> Result<String, String> {
    use axum::extract::State as AxumState;
    use axum::response::Json as ResponseJson;
    
    let result = backend_activate(AxumState(state)).await;
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

/// Get eltord status using backend logic - thin wrapper for Tauri
pub async fn get_eltord_status_wrapper(state: AppState) -> EltordStatusResponse {
    use axum::extract::State as AxumState;
    use axum::response::Json as ResponseJson;
    
    let result = backend_status(AxumState(state)).await;
    match result {
        ResponseJson(status_response) => status_response,
    }
}

/// Get a log receiver for listening to log events
pub fn get_log_receiver(state: &AppState) -> broadcast::Receiver<LogEntry> {
    state.log_sender.subscribe()
}
