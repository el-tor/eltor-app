use axum::{
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};

use crate::state::{AppState, MessageResponse};

// Placeholder wallet handlers
async fn get_wallet_balance() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Wallet balance endpoint - not implemented yet".to_string(),
    })
}

async fn send_payment() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Send payment endpoint - not implemented yet".to_string(),
    })
}

async fn receive_payment() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Receive payment endpoint - not implemented yet".to_string(),
    })
}

async fn get_wallet_status() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Wallet status endpoint - not implemented yet".to_string(),
    })
}

// Create wallet routes
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/wallet/balance", get(get_wallet_balance))
        .route("/api/wallet/send", post(send_payment))
        .route("/api/wallet/receive", post(receive_payment))
        .route("/api/wallet/status", get(get_wallet_status))
}