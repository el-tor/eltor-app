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
    state::{AppState, MessageResponse},
    torrc_parser::{
        get_all_payment_lightning_configs, modify_payment_lightning_config, NodeType, Operation,
    }, PathConfig,
};

// Request types for lightning config management
#[derive(Debug, Deserialize)]
pub struct UpsertLightningConfigRequest {
    pub node_type: String, // "phoenixd", "cln", "lnd"
    pub url: String,
    pub password: String, // Can be password, rune, or macaroon depending on node_type
    pub set_as_default: bool,
    pub is_embedded: Option<bool>, // Indicates if this config is for an embedded Phoenix instance
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
    pub is_embedded: Option<bool>, // Indicates if this config is for an embedded Phoenix instance
}

#[derive(Debug, Serialize)]
pub struct ListLightningConfigsResponse {
    pub configs: Vec<LightningConfigResponse>,
}

// Helper function to get the current lightning node from torrc
// This ensures we always use the latest configuration
async fn get_current_lightning_node() -> Result<crate::lightning::LightningNode, String> {
    let path_config = PathConfig::new()?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);
    crate::lightning::LightningNode::from_torrc(&torrc_path)
        .map_err(|e| format!("Failed to load wallet: {}", e))
}

// Get node information
async fn get_node_info(
    State(_state): State<AppState>,
) -> Result<ResponseJson<NodeInfoResponse>, (StatusCode, String)> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => match node.get_node_info().await {
            Ok(info) => Ok(ResponseJson(info)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get node info: {}", e),
            )),
        },
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("No lightning node configured: {}", e),
        )),
    }
}

// Create an invoice (receive payment)
async fn create_invoice(
    State(_state): State<AppState>,
    Json(request): Json<CreateInvoiceRequest>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => match node.create_invoice(request).await {
            Ok(invoice) => Ok(ResponseJson(invoice)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create invoice: {}", e),
            )),
        },
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("No lightning node configured: {}", e),
        )),
    }
}

// Pay an invoice (send payment)
async fn pay_invoice(
    State(_state): State<AppState>,
    Json(request): Json<PayInvoiceRequest>,
) -> Result<ResponseJson<PayInvoiceResponse>, (StatusCode, String)> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => match node.pay_invoice(request).await {
            Ok(payment) => Ok(ResponseJson(payment)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to pay invoice: {}", e),
            )),
        },
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("No lightning node configured: {}", e),
        )),
    }
}

// Get wallet status (simplified node info)
async fn get_wallet_status(State(_state): State<AppState>) -> ResponseJson<MessageResponse> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => {
            let status = format!("Lightning wallet connected ({})", node.node_type());
            ResponseJson(MessageResponse { message: status })
        }
        Err(_) => ResponseJson(MessageResponse {
            message: "Lightning wallet not connected".to_string(),
        }),
    }
}

// Get wallet transactions
async fn get_wallet_transactions(
    State(_state): State<AppState>,
) -> Result<ResponseJson<ListTransactionsResponse>, (StatusCode, String)> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => {
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
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("No lightning node configured: {}", e),
        )),
    }
}

// Get a BOLT12 offer
async fn get_offer(
    State(_state): State<AppState>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    // Get the current lightning node from torrc to ensure we use the latest config
    match get_current_lightning_node().await {
        Ok(node) => match node.get_offer().await {
            Ok(response) => {
                Ok(ResponseJson(response))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create invoice: {}", e),
            )),
        },
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("No lightning node configured: {}", e),
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
    let path_config = PathConfig::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get path config: {}", e),
        )
    })?;
    path_config.ensure_torrc_files().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to ensure torrc files: {}", e),
        )
    })?;
    let torrc_path = path_config.get_torrc_path(None);

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
    let path_config = PathConfig::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get path config: {}", e),
        )
    })?;
    path_config.ensure_torrc_files().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to ensure torrc files: {}", e),
        )
    })?;
    let torrc_path = path_config.get_torrc_path(None);

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
    let path_config = PathConfig::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get path config: {}", e),
        )
    })?;
    path_config.ensure_torrc_files().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to ensure torrc files: {}", e),
        )
    })?;
    let torrc_path = path_config.get_torrc_path(None);

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
                        node_type: config.node_type.clone(),
                        url: config.url.clone(),
                        password_type: password_type.to_string(),
                        password: config.password, // Include the actual credential
                        is_default: config.is_default,
                        is_embedded: Some(
                            config.node_type == "phoenixd" && 
                            (config.url == "http://127.0.0.1:9740" || config.url == "http://localhost:9740")
                        ), // Detect embedded Phoenix by URL
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
