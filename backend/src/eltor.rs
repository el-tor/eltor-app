use log::{info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// Individual process handle for tracking eltor library tasks
#[derive(Debug)]
pub struct EltorProcessHandle {
    #[allow(dead_code)]
    task_handle: tokio::task::JoinHandle<()>,
    abort_handle: tokio::task::AbortHandle,
    mode: EltorMode,
}

impl EltorProcessHandle {
    async fn stop(&mut self) -> Result<(), String> {
        info!("🛑 Stopping {} process", self.mode);
        
        // ONLY abort the task - NO process killing at all
        self.abort_handle.abort();
        
        // Brief wait for task to abort
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        info!("✅ {} process stopped", self.mode);
        Ok(())
    }
}

/// Eltor mode state
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
            EltorMode::Both => "both", // Pass "both" to eltor lib
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
            EltorMode::Both => "torrc.relay",
        }
    }

    // TODO get from torrc file
    pub fn get_control_port(&self) -> &str {
        match self {
            EltorMode::Client => "9992",
            EltorMode::Relay => "7781",
            EltorMode::Both => "7781",
        }
    }
}

impl std::fmt::Display for EltorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Core eltor management with separate process tracking
pub struct EltorManager {
    pub state: Arc<RwLock<AppState>>,
    pub path_config: PathConfig,
    pub client_process: Arc<RwLock<Option<EltorProcessHandle>>>,
    pub relay_process: Arc<RwLock<Option<EltorProcessHandle>>>,
}

impl EltorManager {
    pub fn new(state: Arc<RwLock<AppState>>, path_config: PathConfig) -> Self {
        Self {
            state,
            path_config,
            client_process: Arc::new(RwLock::new(None)),
            relay_process: Arc::new(RwLock::new(None)),
        }
    }

    /// Activate eltor with the specified parameters
    pub async fn activate(&self, params: EltorActivateParams) -> Result<String, String> {
        match params.mode {
            EltorMode::Client => {
                let mut client_guard = self.client_process.write().await;
                if client_guard.is_some() {
                    return Ok("Eltor client is already running".to_string());
                }

                let handle = self.start_eltor_process(EltorMode::Client).await?;
                *client_guard = Some(handle);
                Ok("Eltor client activated".to_string())
            }
            EltorMode::Relay => {
                let mut relay_guard = self.relay_process.write().await;
                if relay_guard.is_some() {
                    return Ok("Eltor relay is already running".to_string());
                }

                let handle = self.start_eltor_process(EltorMode::Relay).await?;
                *relay_guard = Some(handle);
                Ok("Eltor relay activated".to_string())
            }
            EltorMode::Both => {
                // For "both" mode, we only start one process with mode "both"
                // The eltor library handles running both client and relay within the same process
                let mut relay_guard = self.relay_process.write().await;
                if relay_guard.is_some() {
                    return Ok("Eltor both mode is already running".to_string());
                }

                let handle = self.start_eltor_process(EltorMode::Both).await?;
                *relay_guard = Some(handle);
                Ok("Eltor both mode activated".to_string())
            }
        }
    }

    /// Deactivate eltor for the specified mode
    pub async fn deactivate(&self, params: EltorDeactivateParams) -> Result<String, String> {
        match params.mode {
            EltorMode::Client => {
                let mut client_guard = self.client_process.write().await;
                if let Some(mut handle) = client_guard.take() {
                    // Stop the task (this aborts it immediately)
                    handle.stop().await?;
                    // Clean up any residual Tor state
                    self.cleanup_tor_port(EltorMode::Client).await;
                    Ok("Eltor client deactivated".to_string())
                } else {
                    Ok("Eltor client is not running".to_string())
                }
            }
            EltorMode::Relay => {
                let mut relay_guard = self.relay_process.write().await;
                if let Some(mut handle) = relay_guard.take() {
                    // Stop the task (this aborts it immediately)
                    handle.stop().await?;
                    // Clean up any residual Tor state  
                    self.cleanup_tor_port(EltorMode::Relay).await;
                    Ok("Eltor relay deactivated".to_string())
                } else {
                    Ok("Eltor relay is not running".to_string())
                }
            }
            EltorMode::Both => {
                // For "both" mode, we only have one process in the relay slot
                let mut relay_guard = self.relay_process.write().await;
                if let Some(mut handle) = relay_guard.take() {
                    // Stop the task (this aborts it immediately)
                    handle.stop().await?;
                    // Clean up any residual Tor state  
                    self.cleanup_tor_port(EltorMode::Both).await;
                    Ok("Eltor both mode deactivated".to_string())
                } else {
                    Ok("Eltor both mode is not running".to_string())
                }
            }
        }
    }

    /// Start a new eltor library process as Tokio task
    async fn start_eltor_process(&self, mode: EltorMode) -> Result<EltorProcessHandle, String> {
        let torrc_file = mode.get_torrc_file().to_string();
        let torrc_path = self.path_config.get_torrc_path(Some(&torrc_file));

        info!("🚀 Starting eltor {} as Tokio task with torrc: {:?}", mode, torrc_path);

        // Clean up any residual state first
        info!("🧹 Pre-start cleanup for {} mode", mode);
        self.cleanup_tor_port(mode.clone()).await;

        let mode_str = mode.to_string().to_string();
        let torrc_path_str = torrc_path.to_string_lossy().to_string();

        // Spawn the eltor library as an abortable Tokio task
        let task_handle = tokio::spawn(async move {
            let args = vec![
                "eltord".to_string(),
                mode_str.clone(),
                "-f".to_string(),
                torrc_path_str.clone(),
                "-pw".to_string(),
                "password1234_".to_string(), // TODO read password from env or config
            ];

            info!("🚀 Task starting eltor library {} with args: {:?}", mode_str, args);

            // Run eltor library - this will block until shutdown or abort
            eltor::run_with_args(args).await;
            
            info!("✅ Eltor {} library completed", mode_str);
        });

        // Get the abort handle
        let abort_handle = task_handle.abort_handle();

        // Brief delay to let it start
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(EltorProcessHandle {
            task_handle,
            abort_handle,
            mode,
        })
    }

    /// Cleanup Tor daemon for specific mode - MINIMAL and SAFE cleanup
    async fn cleanup_tor_port(&self, mode: EltorMode) {
        let port = mode.get_control_port();
        
        info!("🧹 Minimal cleanup for {} mode", mode);
        
        // ONLY try Tor control commands - NO process killing!
        if let Err(e) = self.send_tor_shutdown_on_port(port).await {
            warn!("⚠️ Failed to send Tor shutdown command on port {}: {}", port, e);
        }
        
        // For Both mode, also try cleaning up client port
        if mode == EltorMode::Both {
            if let Err(e) = self.send_tor_shutdown_on_port("9992").await {
                warn!("⚠️ Failed to send Tor shutdown command on client port 9992: {}", e);
            }
        }
        
        // Wait a moment for Tor to process shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        info!("✅ Minimal cleanup completed for {} mode", mode);
    }
    

    /// Send a SHUTDOWN command to a specific Tor control port
    async fn send_tor_shutdown_on_port(
        &self,
        port: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        info!("🔌 Connecting to Tor control port {} to send shutdown...", port);

        let mut stream = match tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            TcpStream::connect(format!("127.0.0.1:{}", port))
        ).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                warn!("⚠️ Could not connect to Tor control port {}: {}", port, e);
                return Err(e.into());
            }
            Err(_) => {
                warn!("⚠️ Timeout connecting to Tor control port {}", port);
                return Err("Connection timeout".into());
            }
        };

        // Authenticate
        let auth_command = "AUTHENTICATE \"password1234_\"\r\n"; // TODO read password from env or config
        stream.write_all(auth_command.as_bytes()).await?;

        let mut buf = vec![0; 1024];
        let n = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            stream.read(&mut buf)
        ).await??;
        let response = String::from_utf8_lossy(&buf[..n]);

        if response.contains("250 OK") {
            let shutdown_command = "SIGNAL SHUTDOWN\r\n";
            stream.write_all(shutdown_command.as_bytes()).await?;
            info!("🛑 Sent shutdown command to Tor on port {}", port);
            
            // Give Tor a moment to process the shutdown command
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        } else {
            warn!("⚠️ Failed to authenticate with Tor control port {}: {}", port, response.trim());
        }

        Ok(())
    }

    /// Get the current status of eltor processes
    pub async fn get_status(&self) -> EltorStatus {
        let client_guard = self.client_process.read().await;
        let relay_guard = self.relay_process.read().await;
        
        let client_running = client_guard.is_some();
        let relay_running = relay_guard.is_some();
        
        // If relay_guard contains a "both" mode process, then both client and relay are running
        let both_mode_running = relay_guard.as_ref()
            .map(|handle| handle.mode == EltorMode::Both)
            .unwrap_or(false);
            
        let running = client_running || relay_running;

        let state = self.state.read().await;
        let recent_logs = {
            let logs = state.recent_logs.lock().unwrap();
            logs.clone().into()
        };

        EltorStatus {
            running,
            client_running: client_running || both_mode_running,
            relay_running,
            recent_logs,
        }
    }
}
