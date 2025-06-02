use axum::{
    extract::State,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
};

use crate::{
    state::{AppState, MessageResponse},
    lightning::{
        NodeInfoResponse, WalletBalanceResponse, CreateInvoiceRequest, 
        CreateInvoiceResponse, PayInvoiceRequest, PayInvoiceResponse,
        ListTransactionsResponse, ListTransactionsParams
    }
};

// Get node information
async fn get_node_info(State(state): State<AppState>) -> Result<ResponseJson<NodeInfoResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            match node.get_node_info().await {
                Ok(info) => Ok(ResponseJson(info)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get node info: {}", e)))
            }
        }
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "Lightning node not initialized".to_string()))
    }
}

// Get wallet balance
async fn get_wallet_balance(State(state): State<AppState>) -> Result<ResponseJson<WalletBalanceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            match node.get_balance().await {
                Ok(balance) => Ok(ResponseJson(balance)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get balance: {}", e)))
            }
        }
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "Lightning node not initialized".to_string()))
    }
}

// Create an invoice (receive payment)
async fn create_invoice(
    State(state): State<AppState>,
    Json(request): Json<CreateInvoiceRequest>
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            match node.create_invoice(request).await {
                Ok(invoice) => Ok(ResponseJson(invoice)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create invoice: {}", e)))
            }
        }
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "Lightning node not initialized".to_string()))
    }
}

// Pay an invoice (send payment)
async fn pay_invoice(
    State(state): State<AppState>,
    Json(request): Json<PayInvoiceRequest>
) -> Result<ResponseJson<PayInvoiceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            match node.pay_invoice(request).await {
                Ok(payment) => Ok(ResponseJson(payment)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to pay invoice: {}", e)))
            }
        }
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "Lightning node not initialized".to_string()))
    }
}

// Get wallet status (simplified node info)
async fn get_wallet_status(State(state): State<AppState>) -> ResponseJson<MessageResponse> {
    match &state.lightning_node {
        Some(node) => {
            let status = format!("Lightning wallet connected ({})", node.node_type());
            ResponseJson(MessageResponse { message: status })
        }
        None => ResponseJson(MessageResponse {
            message: "Lightning wallet not connected".to_string(),
        })
    }
}

// Get wallet transactions
async fn get_wallet_transactions(State(state): State<AppState>) -> Result<ResponseJson<ListTransactionsResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            // Use basic parameters - matching the required fields
            let params = ListTransactionsParams {
                payment_hash: None, // Get all transactions
                from: 0,
                limit: 10,
            };
            
            match node.list_transactions(params).await {
                Ok(transactions) => Ok(ResponseJson(transactions)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get transactions: {}", e)))
            }
        }
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "Lightning node not initialized".to_string()))
    }
}

// Create wallet routes
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/wallet/info", get(get_node_info))
        .route("/api/wallet/balance", get(get_wallet_balance))
        .route("/api/wallet/invoice", post(create_invoice))
        .route("/api/wallet/pay", post(pay_invoice))
        .route("/api/wallet/status", get(get_wallet_status))
        .route("/api/wallet/transactions", get(get_wallet_transactions))
}