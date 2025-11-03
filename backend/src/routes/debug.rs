use axum::{extract::State, response::Json as ResponseJson, routing::get, Router};
use serde_json::Value;

use crate::{debug_info::DebugInfo, paths::PathConfig, state::AppState};

/// Get debug information
#[axum::debug_handler(state = AppState)]
pub async fn get_debug_info(State(_state): State<AppState>) -> Result<ResponseJson<Value>, String> {
    // Create PathConfig for web environment
    let path_config = PathConfig::new().map_err(|e| format!("Failed to create path config: {}", e))?;
    
    // Create debug info
    let debug_info = DebugInfo::with_path_config(path_config).await
        .map_err(|e| format!("Failed to create debug info: {}", e))?;
    
    // Convert to JSON value
    let json_value = serde_json::to_value(&debug_info)
        .map_err(|e| format!("Failed to serialize debug info: {}", e))?;
    
    Ok(ResponseJson(json_value))
}

/// Create debug routes
pub fn create_routes() -> Router<AppState> {
    Router::new().route("/api/debug", get(get_debug_info))
}
