use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;

// Re-export LNI types for easier use
pub use lni::cln::{ClnConfig, ClnNode};
pub use lni::lnd::{LndConfig, LndNode};
pub use lni::phoenixd::{PhoenixdConfig, PhoenixdNode};
pub use lni::types::*;

// Import the LightningNode trait to use its methods
use lni::LightningNode as LniTrait;

// Import torrc parser
use crate::torrc_parser::parse_lightning_config_from_torrc;

/// Transaction response structure matching frontend expectations
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub payment_hash: String,
    pub created_at: i64,
    pub amount_msats: i64,
    pub preimage: Option<String>,
    pub payer_note: Option<String>,
    pub settled_at: Option<i64>,
}

/// Response structure for listing transactions
#[derive(Debug, Serialize, Deserialize)]
pub struct ListTransactionsResponse {
    pub transactions: Vec<TransactionResponse>,
}

/// Lightning Node wrapper that uses trait objects for dynamic dispatch
pub struct LightningNode {
    inner: Arc<dyn LniTrait + Send + Sync>,
    node_type: &'static str,
}

impl Clone for LightningNode {
    fn clone(&self) -> Self {
        LightningNode {
            inner: self.inner.clone(),
            node_type: self.node_type,
        }
    }
}

/// Configuration for creating a lightning node connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_type: String,
    pub url: String,
    pub auth_token: Option<String>, // macaroon for LND, rune for CLN, password for Phoenixd
    pub socks5_proxy: Option<String>,
    pub accept_invalid_certs: Option<bool>,
}

/// Response structure for node info - includes raw NodeInfo plus node_type
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    #[serde(flatten)]
    pub node_info: lni::NodeInfo,
    pub node_type: String,
}

/// Response structure for wallet balance
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletBalanceResponse {
    pub total_balance_sats: u64,
    pub confirmed_balance_sats: u64,
    pub unconfirmed_balance_sats: u64,
    pub locked_balance_sats: Option<u64>,
}

/// Request structure for creating an invoice
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount_sats: Option<u64>,
    pub description: Option<String>,
    pub expiry_seconds: Option<u64>,
}

/// Response structure for created invoice
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceResponse {
    pub payment_request: String,
    pub payment_hash: String,
    pub amount_sats: Option<u64>,
    pub expiry: Option<u64>,
}

/// Request structure for paying an invoice
#[derive(Debug, Serialize, Deserialize)]
pub struct PayInvoiceRequest {
    pub payment_request: String,
    pub fee_limit_percentage: Option<f64>,
    pub timeout_seconds: Option<u64>,
}

/// Response structure for payment result
#[derive(Debug, Serialize, Deserialize)]
pub struct PayInvoiceResponse {
    pub payment_hash: String,
    pub payment_preimage: Option<String>,
    pub amount_paid_sats: u64,
    pub fee_paid_sats: u64,
    pub status: String, // "succeeded", "failed", "pending"
}

impl LightningNode {
    /// Create a new lightning node connection based on torrc configuration
    pub fn from_torrc<P: AsRef<std::path::Path>>(torrc_path: P) -> Result<Self, String> {
        let accept_invalid_certs = Some(env::var("ACCEPT_INVALID_CERTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true));
        match parse_lightning_config_from_torrc(&torrc_path)? {
            Some(config) => {
                match config.node_type.as_str() {
                    "phoenixd" => {
                        let phoenixd_config = PhoenixdConfig {
                            url: config.url,
                            password: config.password,
                            socks5_proxy: env::var("SOCKS5_PROXY").ok(),
                            accept_invalid_certs,
                            ..Default::default()
                        };

                        let node = PhoenixdNode::new(phoenixd_config);
                        Ok(LightningNode {
                            inner: Arc::new(node),
                            node_type: "phoenixd",
                        })
                    }
                    "lnd" => {
                        let lnd_config = LndConfig {
                            url: config.url,
                            macaroon: config.password, // For LND, password field contains macaroon
                            socks5_proxy: env::var("SOCKS5_PROXY").ok(),
                            accept_invalid_certs,
                            ..Default::default()
                        };

                        let node = LndNode::new(lnd_config);
                        Ok(LightningNode {
                            inner: Arc::new(node),
                            node_type: "lnd",
                        })
                    }
                    "cln" => {
                        let cln_config = ClnConfig {
                            url: config.url,
                            rune: config.password, // For CLN, password field contains rune
                            socks5_proxy: env::var("SOCKS5_PROXY").ok(),
                            accept_invalid_certs,
                            ..Default::default()
                        };

                        let node = ClnNode::new(cln_config);
                        Ok(LightningNode {
                            inner: Arc::new(node),
                            node_type: "cln",
                        })
                    }
                    _ => Err(format!(
                        "Unsupported node type from torrc: {}",
                        config.node_type
                    )),
                }
            }
            None => Err("No PaymentLightningNodeConfig found in torrc file".to_string()),
        }
    }

    /// Get node information (async to handle blocking LNI calls)
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse, String> {
        let inner = self.inner.clone();
        let info = tokio::task::spawn_blocking(move || inner.get_info())
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to get node info: {:?}", e))?;

        Ok(NodeInfoResponse {
            node_info: info,
            node_type: self.node_type.to_string(),
        })
    }

    /// Create an invoice (async to handle blocking LNI calls)
    pub async fn create_invoice(
        &self,
        request: CreateInvoiceRequest,
    ) -> Result<CreateInvoiceResponse, String> {
        let params = CreateInvoiceParams {
            invoice_type: InvoiceType::Bolt11,
            amount_msats: request.amount_sats.map(|sats| sats as i64 * 1000),
            description: request.description,
            expiry: request.expiry_seconds.map(|s| s as i64),
            ..Default::default()
        };

        let inner = self.inner.clone();
        let transaction = tokio::task::spawn_blocking(move || inner.create_invoice(params))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to create invoice: {:?}", e))?;

        Ok(CreateInvoiceResponse {
            payment_request: transaction.invoice,
            payment_hash: transaction.payment_hash,
            amount_sats: Some((transaction.amount_msats / 1000) as u64),
            expiry: None, // Not available in current Transaction
        })
    }

    /// Pay an invoice (async to handle blocking LNI calls)
    pub async fn pay_invoice(
        &self,
        request: PayInvoiceRequest,
    ) -> Result<PayInvoiceResponse, String> {
        let params = PayInvoiceParams {
            invoice: request.payment_request.clone(),
            fee_limit_percentage: request.fee_limit_percentage,
            allow_self_payment: Some(true),
            ..Default::default()
        };

        let inner = self.inner.clone();
        let response = tokio::task::spawn_blocking(move || inner.pay_invoice(params))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to pay invoice: {:?}", e))?;

        Ok(PayInvoiceResponse {
            payment_hash: response.payment_hash,
            payment_preimage: Some(response.preimage),
            amount_paid_sats: 0, // Amount not available in PayInvoiceResponse
            fee_paid_sats: (response.fee_msats / 1000) as u64,
            status: "succeeded".to_string(), // Assume success if no error
        })
    }

    /// List transactions (async to handle blocking LNI calls)
    pub async fn list_transactions(
        &self,
        params: ListTransactionsParams,
    ) -> Result<ListTransactionsResponse, String> {
        let inner = self.inner.clone();
        let transactions = tokio::task::spawn_blocking(move || inner.list_transactions(params))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to list transactions: {:?}", e))?;

        let responses: Vec<TransactionResponse> = transactions
            .into_iter()
            .map(|tx| TransactionResponse {
                payment_hash: tx.payment_hash,
                created_at: tx.created_at,
                amount_msats: tx.amount_msats,
                preimage: Some(tx.preimage),
                payer_note: None, // Not available in current Transaction
                settled_at: Some(tx.settled_at),
            })
            .collect();

        Ok(ListTransactionsResponse {
            transactions: responses,
        })
    }

    /// Create an invoice (async to handle blocking LNI calls)
    pub async fn get_offer(&self) -> Result<CreateInvoiceResponse, String> {
        let params = CreateInvoiceParams {
            invoice_type: InvoiceType::Bolt12,
            amount_msats: None,
            description: Some("El Tor Offer".to_string()),
            ..Default::default()
        };

        let inner = self.inner.clone();
        let transaction = tokio::task::spawn_blocking(move || inner.create_invoice(params))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to create invoice: {:?}", e))?;

        Ok(CreateInvoiceResponse {
            payment_request: transaction.invoice,
            payment_hash: transaction.payment_hash,
            amount_sats: Some((transaction.amount_msats / 1000) as u64),
            expiry: None,
        })
    }

    /// Get node type as string
    pub fn node_type(&self) -> &'static str {
        self.node_type
    }
}
