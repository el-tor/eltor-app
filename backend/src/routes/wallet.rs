use axum::{
    extract::State,
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    lightning::{
        CreateInvoiceRequest, CreateInvoiceResponse, ListTransactionsParams,
        ListTransactionsResponse, NodeInfoResponse, PayInvoiceRequest, PayInvoiceResponse,
    },
    routes::eltor::get_bin_dir,
    state::{AppState, MessageResponse},
    torrc_parser::{
        get_all_payment_lightning_configs, modify_payment_lightning_config, NodeType, Operation,
    },
};

// Request types for lightning config management
#[derive(Debug, Deserialize)]
pub struct UpsertLightningConfigRequest {
    pub node_type: String, // "phoenixd", "cln", "lnd"
    pub url: String,
    pub password: String, // Can be password, rune, or macaroon depending on node_type
    pub set_as_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeleteLightningConfigRequest {
    pub node_type: String,   // "phoenixd", "cln", "lnd"
    pub url: Option<String>, // If None, deletes first match of node_type
}

#[derive(Debug, Serialize)]
pub struct LightningConfigResponse {
    pub node_type: String,
    pub url: String,
    pub password_type: String, // "password", "rune", or "macaroon"
    pub password: String,      // The actual credential value
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct ListLightningConfigsResponse {
    pub configs: Vec<LightningConfigResponse>,
}

// Get node information
async fn get_node_info(
    State(state): State<AppState>,
) -> Result<ResponseJson<NodeInfoResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => match node.get_node_info().await {
            Ok(info) => Ok(ResponseJson(info)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get node info: {}", e),
            )),
        },
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Lightning node not initialized".to_string(),
        )),
    }
}

// Create an invoice (receive payment)
async fn create_invoice(
    State(state): State<AppState>,
    Json(request): Json<CreateInvoiceRequest>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => match node.create_invoice(request).await {
            Ok(invoice) => Ok(ResponseJson(invoice)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create invoice: {}", e),
            )),
        },
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Lightning node not initialized".to_string(),
        )),
    }
}

// Pay an invoice (send payment)
async fn pay_invoice(
    State(state): State<AppState>,
    Json(request): Json<PayInvoiceRequest>,
) -> Result<ResponseJson<PayInvoiceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => match node.pay_invoice(request).await {
            Ok(payment) => Ok(ResponseJson(payment)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to pay invoice: {}", e),
            )),
        },
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Lightning node not initialized".to_string(),
        )),
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
        }),
    }
}

// Get wallet transactions
async fn get_wallet_transactions(
    State(state): State<AppState>,
) -> Result<ResponseJson<ListTransactionsResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => {
            // Use basic parameters - matching the required fields
            let params = ListTransactionsParams {
                payment_hash: None, // Get all transactions
                from: 0,
                limit: 1000,
                search: None,
            };

            match node.list_transactions(params).await {
                Ok(transactions) => Ok(ResponseJson(transactions)),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get transactions: {}", e),
                )),
            }
        }
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Lightning node not initialized".to_string(),
        )),
    }
}

// Get a BOLT12 offer
async fn get_offer(
    State(state): State<AppState>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    match &state.lightning_node {
        Some(node) => match node.get_offer().await {
            Ok(invoice) => Ok(ResponseJson(invoice)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create invoice: {}", e),
            )),
        },
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Lightning node not initialized".to_string(),
        )),
    }
}

// Upsert lightning configuration
async fn upsert_lightning_config(
    State(_state): State<AppState>,
    Json(request): Json<UpsertLightningConfigRequest>,
) -> Result<ResponseJson<MessageResponse>, (StatusCode, String)> {
    // Parse node type
    let node_type = match request.node_type.as_str() {
        "phoenixd" => NodeType::Phoenixd,
        "cln" => NodeType::Cln,
        "lnd" => NodeType::Lnd,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid node type. Must be 'phoenixd', 'cln', or 'lnd'".to_string(),
            ))
        }
    };

    // Get torrc file path
    let bin_dir = get_bin_dir();
    let torrc_path = bin_dir.join("data").join("torrc");

    // Modify the payment lightning config
    match modify_payment_lightning_config(
        &torrc_path,
        Operation::Upsert,
        node_type,
        Some(request.url.clone()),
        Some(request.password.clone()),
        request.set_as_default,
    ) {
        Ok(_) => Ok(ResponseJson(MessageResponse {
            message: format!(
                "Successfully upserted {} lightning config for {}",
                request.node_type, request.url
            ),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to upsert lightning config: {}", e),
        )),
    }
}

// Delete lightning configuration
async fn delete_lightning_config(
    State(_state): State<AppState>,
    Json(request): Json<DeleteLightningConfigRequest>,
) -> Result<ResponseJson<MessageResponse>, (StatusCode, String)> {
    // Parse node type
    let node_type = match request.node_type.as_str() {
        "phoenixd" => NodeType::Phoenixd,
        "cln" => NodeType::Cln,
        "lnd" => NodeType::Lnd,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid node type. Must be 'phoenixd', 'cln', or 'lnd'".to_string(),
            ))
        }
    };

    // Get torrc file path
    let bin_dir = get_bin_dir();
    let torrc_path = bin_dir.join("data").join("torrc");

    // Delete the lightning config
    match modify_payment_lightning_config(
        &torrc_path,
        Operation::Delete,
        node_type,
        request.url.clone(),
        None,
        false,
    ) {
        Ok(_) => {
            let message = match request.url {
                Some(url) => format!(
                    "Successfully deleted {} lightning config for {}",
                    request.node_type, url
                ),
                None => format!(
                    "Successfully deleted {} lightning config",
                    request.node_type
                ),
            };
            Ok(ResponseJson(MessageResponse { message }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete lightning config: {}", e),
        )),
    }
}

// List all lightning configurations
async fn list_lightning_configs(
    State(_state): State<AppState>,
) -> Result<ResponseJson<ListLightningConfigsResponse>, (StatusCode, String)> {
    // Get torrc file path
    let bin_dir = get_bin_dir();
    let torrc_path = bin_dir.join("data").join("torrc");

    // Get all payment lightning configs
    match get_all_payment_lightning_configs(&torrc_path) {
        Ok(configs) => {
            let response_configs: Vec<LightningConfigResponse> = configs
                .into_iter()
                .map(|config| {
                    // Determine password type based on node type
                    let password_type = match config.node_type.as_str() {
                        "phoenixd" => "password",
                        "cln" => "rune",
                        "lnd" => "macaroon",
                        _ => "password", // fallback
                    };

                    LightningConfigResponse {
                        node_type: config.node_type,
                        url: config.url,
                        password_type: password_type.to_string(),
                        password: config.password, // Include the actual credential
                        is_default: config.is_default,
                    }
                })
                .collect();

            Ok(ResponseJson(ListLightningConfigsResponse {
                configs: response_configs,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list lightning configs: {}", e),
        )),
    }
}

// Create wallet routes
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/wallet/info", get(get_node_info))
        .route("/api/wallet/invoice", post(create_invoice))
        .route("/api/wallet/offer", post(get_offer))
        .route("/api/wallet/pay", post(pay_invoice))
        .route("/api/wallet/status", get(get_wallet_status))
        .route("/api/wallet/transactions", get(get_wallet_transactions))
        .route("/api/wallet/config", put(upsert_lightning_config))
        .route("/api/wallet/config", delete(delete_lightning_config))
        .route("/api/wallet/configs", get(list_lightning_configs))
}
