use futures::future::AbortHandle;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::paths::PathConfig;
use crate::state::{AppState, LogEntry};

/// Parameters for eltor activation
#[derive(Debug, Clone)]
pub struct EltorActivateParams {
    pub mode: EltorMode,
}

/// Parameters for eltor deactivation
#[derive(Debug, Clone)]
pub struct EltorDeactivateParams {
    pub mode: EltorMode,
}

/// Status information for eltor processes
#[derive(Debug, Clone, serde::Serialize)]
pub struct EltorStatus {
    pub running: bool,
    pub client_running: bool,
    pub relay_running: bool,
    pub recent_logs: Vec<LogEntry>,
}

/// Result type for eltor operations
pub type EltorResult<T> = Result<T, String>;

/// Eltor mode state - tracks user intentions
#[derive(Debug, Clone, PartialEq)]
pub enum EltorMode {
    Client,
    Relay,
    Both,
}

impl EltorMode {
    pub fn to_string(&self) -> &str {
        match self {
            EltorMode::Client => "client",
            EltorMode::Relay => "relay",
            EltorMode::Both => "both",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "client" => Ok(EltorMode::Client),
            "relay" => Ok(EltorMode::Relay),
            "both" => Ok(EltorMode::Both),
            _ => Err(format!("Invalid mode: {}", s)),
        }
    }

    pub fn get_torrc_file(&self) -> &str {
        match self {
            EltorMode::Client => "torrc",
            EltorMode::Relay => "torrc.relay",
            EltorMode::Both => "torrc.relay", // Use relay torrc for both mode
        }
    }
}

impl std::fmt::Display for EltorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Core eltor management with single instance mode tracking
pub struct EltorManager {
    pub state: Arc<RwLock<AppState>>,
    pub path_config: PathConfig,
    pub current_mode: Arc<RwLock<Option<EltorMode>>>,
    pub task_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    pub abort_handle: Arc<RwLock<Option<AbortHandle>>>,
}

impl EltorManager {
    pub fn new(state: Arc<RwLock<AppState>>, path_config: PathConfig) -> Self {
        Self {
            state,
            path_config,
            current_mode: Arc::new(RwLock::new(None)),
            task_handle: Arc::new(RwLock::new(None)),
            abort_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Activate eltor with the specified parameters
    /// Simple sequential approach: if client is running and relay is requested, deactivate client then activate "both"
    pub async fn activate(&self, params: EltorActivateParams) -> Result<String, String> {
        let current_mode = self.current_mode.read().await.clone();

        // Handle the simple case: if client is already running and we want to activate relay,
        // stop client completely, then activate "both" mode
        if let Some(EltorMode::Client) = current_mode {
            if params.mode == EltorMode::Relay {
                info!("🔄 Client is running and relay requested - stopping client completely");
                self.stop_current_instance().await?;

                info!("🚀 Now activating both client and relay together");
                self.start_instance(EltorMode::Both).await?;
                return Ok("Eltor activated in both mode".to_string());
            }
        }

        // Handle the simple case: if relay is already running and we want to activate client,
        // stop relay completely, then activate "both" mode
        if let Some(EltorMode::Relay) = current_mode {
            if params.mode == EltorMode::Client {
                info!("🔄 Relay is running and client requested - stopping relay completely");
                self.stop_current_instance().await?;

                info!("🚀 Now activating both client and relay together");
                self.start_instance(EltorMode::Both).await?;
                return Ok("Eltor activated in both mode".to_string());
            }
        }

        // Determine the new mode for other cases
        let new_mode = match (&current_mode, params.mode) {
            (None, EltorMode::Client) => EltorMode::Client,
            (None, EltorMode::Relay) => EltorMode::Relay,
            (Some(EltorMode::Client), EltorMode::Client) => {
                return Ok("Eltor client is already active".to_string());
            }
            (Some(EltorMode::Relay), EltorMode::Relay) => {
                return Ok("Eltor relay is already active".to_string());
            }
            (Some(EltorMode::Both), _) => {
                return Ok("Eltor is already running in both modes".to_string());
            }
            _ => return Err(format!("Invalid mode")),
        };

        // For simple cases, just stop existing and start new
        self.stop_current_instance().await?;
        let mode_str = new_mode.to_string();
        self.start_instance(new_mode.clone()).await?;

        Ok(format!("Eltor {} activated in {} mode", new_mode, mode_str))
    }

    /// Stop the current eltor instance - simplified approach
    async fn stop_current_instance(&self) -> Result<(), String> {
        info!("🛑 Stopping current eltor instance...");

        // First, abort the current task if it exists
        let mut abort_handle_guard = self.abort_handle.write().await;
        if let Some(abort_handle) = abort_handle_guard.take() {
            info!("🛑 Aborting eltor task...");
            abort_handle.abort();
        } else {
            warn!("⚠️ No abort handle found");
        }

        // Wait for task to complete with reasonable timeout
        let mut task_handle_guard = self.task_handle.write().await;
        if let Some(task_handle) = task_handle_guard.take() {
            info!("⏳ Waiting for eltor task to complete...");
            match tokio::time::timeout(tokio::time::Duration::from_secs(10), task_handle).await {
                Ok(result) => match result {
                    Ok(()) => info!("🏁 Eltor task completed successfully"),
                    Err(e) => {
                        if e.is_cancelled() {
                            info!("🛑 Eltor task was cancelled as expected");
                        } else {
                            warn!("⚠️ Eltor task completed with error: {:?}", e);
                        }
                    }
                },
                Err(_) => {
                    warn!("⚠️ Timeout waiting for eltor task to complete - proceeding anyway");
                    // Don't fail here, just continue
                }
            }
        } else {
            warn!("⚠️ No task handle found");
        }

        // Clear current mode after stopping
        info!("🧹 Clearing current mode state");
        *self.current_mode.write().await = None;

        // Aggressive cleanup: Kill any processes using our ports
        info!("🧹 Performing aggressive port cleanup...");
        self.cleanup_ports().await;

        // Simple delay to allow cleanup
        info!("⏳ Waiting 2 seconds for cleanup...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        info!("✅ Eltor instance stopped successfully");

        Ok(())
    }

    /// Aggressively cleanup any processes using our Tor ports
    async fn cleanup_ports(&self) {
        info!("🧹 Attempting to shutdown embedded Tor daemon...");

        // TODO read ports from torrc and torrc.relay files

        // Try to send SHUTDOWN command to Tor control port
        if let Err(e) = self.send_tor_shutdown_on_port("9992").await {
            warn!("⚠️ Failed to send Tor shutdown command on port 9992: {}", e);
        }

        // Try to send SHUTDOWN command to Tor control relay port
        if let Err(e) = self.send_tor_shutdown_on_port("7781").await {
            warn!("⚠️ Failed to send Tor shutdown command on port 7781: {}", e);
        }
    }


    /// Send a SHUTDOWN command to a specific Tor control port
    async fn send_tor_shutdown_on_port(
        &self,
        port: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        info!(
            "🔌 Connecting to Tor control port {} to send shutdown...",
            port
        );

        // Try to connect to the Tor control port
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;

        // Authenticate with the password
        // TODO remove hard code control port password
        let auth_command = "AUTHENTICATE \"password1234_\"\r\n";
        stream.write_all(auth_command.as_bytes()).await?;

        // Read response
        let mut buf = vec![0; 1024];
        let n = stream.read(&mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);
        info!("🔐 Tor auth response on port {}: {}", port, response.trim());

        if response.contains("250 OK") {
            // Send shutdown command
            info!("📤 Sending SHUTDOWN command to Tor on port {}...", port);
            let shutdown_command = "SIGNAL SHUTDOWN\r\n";
            stream.write_all(shutdown_command.as_bytes()).await?;

            // Read response
            let mut buf = vec![0; 1024];
            let n = stream.read(&mut buf).await?;
            let response = String::from_utf8_lossy(&buf[..n]);
            info!(
                "🛑 Tor shutdown response on port {}: {}",
                port,
                response.trim()
            );
        } else {
            error!("❌ Failed to authenticate with Tor control port {}", port);
        }

        Ok(())
    }

    /// Start a new eltor instance with the specified mode
    async fn start_instance(&self, mode: EltorMode) -> Result<(), String> {
        let torrc_file = mode.get_torrc_file().to_string(); // Clone to avoid borrow issues
        let torrc_path = self.path_config.get_torrc_path(Some(&torrc_file));

        info!("🔍 Looking for torrc file at: {:?}", torrc_path);
        info!(
            "🚀 Starting eltor library with mode: {} using torrc: {}",
            mode.to_string(),
            torrc_file
        );

        let mode_str = mode.to_string().to_string();
        let torrc_path_str = torrc_path.to_string_lossy().to_string();

        // Log the mode being started for debugging
        match &mode {
            EltorMode::Client => info!("🎯 Starting CLIENT mode - logs should appear"),
            EltorMode::Relay => info!("🎯 Starting RELAY mode - logs should appear"),
            EltorMode::Both => info!("🎯 Starting BOTH mode - logs should appear"),
        }

        // Create an abort handle to cleanly stop the task if needed
        let (abort_handle, abort_registration) = futures::future::AbortHandle::new_pair();

        // Create the entire task as an abortable future with proper cleanup
        let task_future = async move {
            // Wrap the entire eltor execution in a result to prevent panics from crashing the backend
            let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
                // Don't initialize env_logger here - use the existing BroadcastLogger from main.rs
                // This ensures all logs go through our custom logger and get streamed to frontend

                // Set args for and where to find the torrc file, then start eltor in mode client, relay or both
                let args = vec![
                    "eltord".to_string(),
                    mode_str.clone(),
                    "-f".to_string(),
                    torrc_path_str.clone(),
                    "-pw".to_string(),
                    "password1234_".to_string(), // TODO read from ENV var
                ];

                info!("🚀 Starting eltor with args: {:?}", args);
                info!("🚀 Launching eltor library in {} mode...", mode_str);
                info!("📍 About to call eltor::run_with_args() for {}", mode_str);

                // Start eltord with the specified mode
                // Note: eltor::run_with_args() will run until cancelled/aborted
                eltor::run_with_args(args).await;
                info!("✅ Eltor {} completed successfully", mode_str);
                Ok(())
            }
            .await;

            if let Err(e) = result {
                error!("❌ Eltor {} task failed: {}", mode_str, e);
            } else {
                info!("✅ Eltor {} task completed without errors", mode_str);
            }

            // Cleanup: Force kill any remaining tor processes that might be stuck
            info!("🧹 Task ending - performing cleanup...");
        };

        // Make the entire task abortable and spawn it
        let abortable_task = futures::future::Abortable::new(task_future, abort_registration);
        let task = tokio::spawn(async move {
            match abortable_task.await {
                Ok(()) => {
                    info!("🏁 Eltor task completed normally");
                }
                Err(_) => {
                    info!("🛑 Eltor task was aborted");
                }
            }
        });

        // Update the current mode and store handles
        *self.current_mode.write().await = Some(mode);
        *self.task_handle.write().await = Some(task);
        *self.abort_handle.write().await = Some(abort_handle);

        // Small delay to allow process to start
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(())
    }

    /// Deactivate eltor for the specified mode
    /// This will determine the new mode based on current state and user intention
    pub async fn deactivate(&self, params: EltorDeactivateParams) -> Result<String, String> {
        info!("🛑 Deactivate request for mode: {}", params.mode);
        let current_mode = self.current_mode.read().await.clone();

        match &current_mode {
            None => Ok("No eltor process is currently running".to_string()),
            Some(mode) => {
                info!("🔍 Current mode: {:?}, deactivating: {}", mode, params.mode);

                // Determine what to do based on current mode and deactivation request
                let new_mode = match (mode, params.mode) {
                    (EltorMode::Client, EltorMode::Client) => {
                        info!("🛑 Stopping client completely");
                        None // Stop everything
                    }
                    (EltorMode::Relay, EltorMode::Relay) => {
                        info!("🛑 Stopping relay completely");
                        None // Stop everything
                    }
                    (EltorMode::Both, EltorMode::Client) => {
                        info!("🔄 Switching from Both to Relay-only");
                        Some(EltorMode::Relay) // Keep relay only
                    }
                    (EltorMode::Both, EltorMode::Relay) => {
                        info!("🔄 Switching from Both to Client-only");
                        Some(EltorMode::Client) // Keep client only
                    }
                    (EltorMode::Client, EltorMode::Relay) => {
                        return Ok("Eltor relay is not currently running".to_string());
                    }
                    (EltorMode::Relay, EltorMode::Client) => {
                        return Ok("Eltor client is not currently running".to_string());
                    }
                    _ => return Err(format!("Invalid mode")),
                };

                // Always stop the current instance first
                info!("🛑 Stopping current eltor instance...");
                self.stop_current_instance().await?;

                // If we need to restart with a different mode, do so
                if let Some(new_mode) = new_mode {
                    info!("🚀 Restarting in {:?} mode", new_mode);
                    self.start_instance(new_mode.clone()).await?;
                    Ok(format!(
                        "Eltor deactivated, now running in {:?} mode",
                        new_mode
                    ))
                } else {
                    info!("✅ All eltor processes stopped");
                    Ok(format!("Eltor deactivated"))
                }
            }
        }
    }

    /// Get the current status of eltor processes
    pub async fn get_status(&self) -> EltorStatus {
        let current_mode = self.current_mode.read().await.clone();
        let state = self.state.read().await;

        let (running, client_running, relay_running) = match &current_mode {
            None => (false, false, false),
            Some(EltorMode::Client) => (true, true, false),
            Some(EltorMode::Relay) => (true, false, true),
            Some(EltorMode::Both) => (true, true, true),
        };

        let recent_logs = {
            let logs = state.recent_logs.lock().unwrap();
            logs.clone().into()
        };

        EltorStatus {
            running,
            client_running,
            relay_running,
            recent_logs,
        }
    }
}

// Legacy compatibility functions for existing lib.rs exports
pub async fn activate_eltor(
    state: Arc<RwLock<AppState>>,
    params: EltorActivateParams,
) -> Result<String, String> {
    let path_config = PathConfig::new()?;
    let manager = EltorManager::new(state, path_config);
    manager.activate(params).await
}

pub async fn deactivate_eltor(
    state: Arc<RwLock<AppState>>,
    params: EltorDeactivateParams,
) -> Result<String, String> {
    let path_config = PathConfig::new()?;
    let manager = EltorManager::new(state, path_config);
    manager.deactivate(params).await
}

pub async fn get_eltor_status(state: Arc<RwLock<AppState>>) -> Result<EltorStatus, String> {
    let path_config = PathConfig::new()?;
    let manager = EltorManager::new(state, path_config);
    Ok(manager.get_status().await)
}

// For backwards compatibility with binary usage (if needed)
// #[allow(unused)]
// pub fn get_bin_dir() -> String {
//     std::env::current_dir()
//         .unwrap()
//         .join("bin")
//         .to_string_lossy()
//         .to_string()
// }
