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

// Helper function to get the current lightning node from app state
// This uses the cached node from state instead of recreating it from torrc every time
async fn get_lightning_node_from_state(state: &AppState) -> Result<crate::lightning::LightningNode, String> {
    // Get the cached lightning node from app state
    let lightning_node_guard = state.lightning_node.lock().unwrap();
    lightning_node_guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Lightning node not initialized in app state".to_string())
}

// Get node information
async fn get_node_info(
    State(state): State<AppState>,
) -> Result<ResponseJson<NodeInfoResponse>, (StatusCode, String)> {
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
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
    State(state): State<AppState>,
    Json(request): Json<CreateInvoiceRequest>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
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
    State(state): State<AppState>,
    Json(request): Json<PayInvoiceRequest>,
) -> Result<ResponseJson<PayInvoiceResponse>, (StatusCode, String)> {
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
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
async fn get_wallet_status(State(state): State<AppState>) -> ResponseJson<MessageResponse> {
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
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
    State(state): State<AppState>,
) -> Result<ResponseJson<ListTransactionsResponse>, (StatusCode, String)> {
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
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
    State(state): State<AppState>,
) -> Result<ResponseJson<CreateInvoiceResponse>, (StatusCode, String)> {
    // Get torrc file paths
    let path_config = PathConfig::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get path config: {}", e),
        )
    })?;
    let torrc_relay_path = path_config.get_torrc_relay_path();
    
    // First, try to get the offer from torrc.relay file
    let existing_offers = crate::torrc_parser::get_torrc_config(&torrc_relay_path, "PaymentBolt12Offer");
    
    if !existing_offers.is_empty() && !existing_offers[0].is_empty() {
        println!("✅ Using existing PaymentBolt12Offer from torrc.relay");
        return Ok(ResponseJson(CreateInvoiceResponse {
            payment_request: existing_offers[0].clone(),
            payment_hash: String::new(), // Not needed for cached offer
            amount_sats: None,
            expiry: None,
        }));
    }
    
    println!("📡 No BOLT12 offer found in torrc.relay, fetching from Lightning node...");
    
    // Get the cached lightning node from app state
    match get_lightning_node_from_state(&state).await {
        Ok(node) => match node.get_offer().await {
            Ok(response) => {
                // Update torrc.relay with the new offer
                if let Err(e) = crate::torrc_parser::update_torrc_config_line(
                    &torrc_relay_path,
                    "PaymentBolt12Offer",
                    &response.payment_request,
                ) {
                    eprintln!("⚠️ Warning: Failed to update PaymentBolt12Offer in torrc.relay: {}", e);
                }
                
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
    State(state): State<AppState>,
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

    // Get torrc file paths
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
    let torrc_relay_path = path_config.get_torrc_relay_path();

    // Check if the node type is changing (before we modify the config)
    let node_type_changed = if request.set_as_default {
        let previous_configs = crate::torrc_parser::get_all_payment_lightning_configs(&torrc_path)
            .unwrap_or_else(|_| Vec::new());
        
        println!("🔍 All configs before update: {:?}", previous_configs);
        
        // Find the current default config
        let previous_default = previous_configs.iter()
            .find(|config| config.is_default);
        
        if let Some(prev_default) = previous_default {
            let prev_type = &prev_default.node_type;
            let new_type = &request.node_type;
            let changed = prev_type != new_type;
            println!("🔍 Previous default node type: '{}' (len: {}), New node type: '{}' (len: {}), Changed: {}", 
                prev_type, prev_type.len(), new_type, new_type.len(), changed);
            println!("🔍 Byte comparison: prev={:?}, new={:?}", prev_type.as_bytes(), new_type.as_bytes());
            changed
        } else {
            println!("🔍 No previous default found, this is the first default config");
            false
        }
    } else {
        false
    };

    // Modify the payment lightning config in torrc (client config)
    match modify_payment_lightning_config(
        &torrc_path,
        Operation::Upsert,
        node_type.clone(),
        Some(request.url.clone()),
        Some(request.password.clone()),
        request.set_as_default,
    ) {
        Ok(_) => {
            // Also update torrc.relay with the same config
            if let Err(e) = modify_payment_lightning_config(
                &torrc_relay_path,
                Operation::Upsert,
                node_type,
                Some(request.url.clone()),
                Some(request.password.clone()),
                request.set_as_default,
            ) {
                println!("⚠️  Failed to update PaymentLightningNodeConfig in torrc.relay: {}", e);
            } else {
                println!("✅ Updated PaymentLightningNodeConfig in torrc.relay");
            }
            
            // If this is being set as default, reload the lightning node in app state
            // and update the BOLT12 offer in torrc.relay
            if request.set_as_default {
                match crate::lightning::LightningNode::from_torrc(&torrc_path) {
                    Ok(new_node) => {
                        // Determine if we should fetch a new offer
                        let should_fetch_new_offer = if node_type_changed {
                            // Always fetch new offer when node type changes
                            true
                        } else {
                            // Check if offer exists in torrc.relay
                            let existing_offers = crate::torrc_parser::get_torrc_config(&torrc_relay_path, "PaymentBolt12Offer");
                            existing_offers.is_empty() || existing_offers.first().map(|s| s.is_empty()).unwrap_or(true)
                        };
                        
                        if should_fetch_new_offer {
                            // Fetch a new offer from the lightning node
                            if node_type_changed {
                                println!("📡 Lightning node type changed, fetching new BOLT12 offer...");
                            } else {
                                println!("📡 No BOLT12 offer found in torrc.relay, fetching from lightning node...");
                            }
                            
                            match new_node.get_offer().await {
                                Ok(offer_response) => {
                                    // Update PaymentBolt12Offer in torrc.relay
                                    if let Err(e) = crate::torrc_parser::update_torrc_config_line(
                                        &torrc_relay_path,
                                        "PaymentBolt12Offer",
                                        &offer_response.payment_request,
                                    ) {
                                        println!("⚠️  Failed to update PaymentBolt12Offer in torrc.relay: {}", e);
                                    } else {
                                        println!("✅ Updated PaymentBolt12Offer in torrc.relay: {}", &offer_response.payment_request);
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Failed to get BOLT12 offer from lightning node: {}", e);
                                }
                            }
                        } else {
                            let existing_offers = crate::torrc_parser::get_torrc_config(&torrc_relay_path, "PaymentBolt12Offer");
                            if let Some(offer) = existing_offers.first() {
                                println!("✅ PaymentBolt12Offer already exists in torrc.relay: {}", offer);
                            }
                        }
                        
                        state.set_lightning_node(new_node);
                        println!("✅ Lightning node reloaded from torrc after upsert");
                    }
                    Err(e) => {
                        println!("⚠️  Failed to reload lightning node: {}", e);
                    }
                }
            }
            
            Ok(ResponseJson(MessageResponse {
                message: format!(
                    "Successfully upserted {} lightning config for {}",
                    request.node_type, request.url
                ),
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to upsert lightning config: {}", e),
        )),
    }
}

// Delete lightning configuration
async fn delete_lightning_config(
    State(state): State<AppState>,
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
    let torrc_relay_path = path_config.get_torrc_relay_path();

    // Delete the lightning config from torrc (client config)
    match modify_payment_lightning_config(
        &torrc_path,
        Operation::Delete,
        node_type.clone(),
        request.url.clone(),
        None,
        false,
    ) {
        Ok(_) => {
            // Also delete from torrc.relay
            if let Err(e) = modify_payment_lightning_config(
                &torrc_relay_path,
                Operation::Delete,
                node_type,
                request.url.clone(),
                None,
                false,
            ) {
                println!("⚠️  Failed to delete PaymentLightningNodeConfig from torrc.relay: {}", e);
            } else {
                println!("✅ Deleted PaymentLightningNodeConfig from torrc.relay");
            }
            
            // After deletion, try to reload the lightning node with any new default
            match crate::lightning::LightningNode::from_torrc(&torrc_path) {
                Ok(new_node) => {
                    state.set_lightning_node(new_node);
                    println!("✅ Lightning node reloaded from torrc after deletion");
                }
                Err(e) => {
                    // It's okay if there's no default config after deletion
                    println!("⚠️  No default lightning node after deletion: {}", e);
                    let mut node_guard = state.lightning_node.lock().unwrap();
                    *node_guard = None;
                }
            }
            
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
