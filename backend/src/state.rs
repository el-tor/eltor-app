use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use crate::lightning::LightningNode;

// Log entry structure
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub source: String, // "stdout" or "stderr"
    pub mode: Option<String>, // "client", "relay", or None for system logs
}

// Wallet state for tracking phoenixd process and configuration
#[derive(Debug, Clone)]
pub struct WalletState {
    pub use_phoenixd_embedded: bool,
    pub phoenixd_process: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl WalletState {
    pub fn new(use_phoenixd_embedded: bool) -> Self {
        Self {
            use_phoenixd_embedded,
            phoenixd_process: Arc::new(Mutex::new(None)),
        }
    }
}

// Shared state for tracking eltord processes and logs
#[derive(Clone)]
pub struct AppState {
    pub client_process: Arc<Mutex<Option<tokio::process::Child>>>,
    pub relay_process: Arc<Mutex<Option<tokio::process::Child>>>,
    pub log_sender: broadcast::Sender<LogEntry>,
    pub recent_logs: Arc<Mutex<VecDeque<LogEntry>>>,
    pub wallet_state: WalletState,
    pub lightning_node: Option<Arc<LightningNode>>,
    pub torrc_file_name: String,
}

impl AppState {
    pub fn new(use_phoenixd_embedded: bool) -> Self {
        let (log_sender, _) = broadcast::channel(1000);
        Self {
            client_process: Arc::new(Mutex::new(None)),
            relay_process: Arc::new(Mutex::new(None)),
            log_sender,
            recent_logs: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            wallet_state: WalletState::new(use_phoenixd_embedded),
            lightning_node: None,
            torrc_file_name: "torrc".to_string(),
        }
    }

    pub fn set_lightning_node(&mut self, node: LightningNode) {
        self.lightning_node = Some(Arc::new(node));
    }

    pub fn add_log(&self, entry: LogEntry) {
        // Add to recent logs with rotation
        {
            let mut logs = self.recent_logs.lock().unwrap();
            if logs.len() >= 100 {
                logs.pop_front();
            }
            logs.push_back(entry.clone());
        }

        // Send to broadcast channel (ignore errors if no receivers)
        let _ = self.log_sender.send(entry);
    }

    pub fn get_recent_logs(&self) -> Vec<LogEntry> {
        self.recent_logs.lock().unwrap().clone().into()
    }
}

// Response structures
#[derive(Serialize)]
pub struct StatusResponse {
    pub connected: bool,
    pub circuit: Option<String>,
}

#[derive(Serialize)]
pub struct EltordStatusResponse {
    pub running: bool,
    pub pid: Option<u32>,
    pub recent_logs: Vec<LogEntry>,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}