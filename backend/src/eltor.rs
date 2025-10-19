use log::{info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::paths::PathConfig;
use crate::state::{AppState, LogEntry};
use crate::torrc_parser;

/// Get the Tor control password from environment variables
///
/// This function looks for different environment variables based on the mode:
/// - For relay mode: APP_ELTOR_TOR_RELAY_CONTROL_PASSWORD
/// - For client mode: APP_ELTOR_TOR_CONTROL_PASSWORD
/// - Fallback: "password1234_" as default
fn get_tor_control_password(mode: &EltorMode) -> String {
    let env_var = match mode {
        EltorMode::Client => "APP_ELTOR_TOR_CONTROL_PASSWORD",
        EltorMode::Relay | EltorMode::Both => "APP_ELTOR_TOR_RELAY_CONTROL_PASSWORD",
    };

    std::env::var(env_var).unwrap_or_else(|_| "password1234_".to_string())
}

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
    tor_pids: Vec<u32>, // Track actual Tor daemon PIDs
}

impl EltorProcessHandle {
    async fn stop(&mut self) -> Result<(), String> {
        info!("🛑 Stopping {} process with {} Tor daemon(s)", self.mode, self.tor_pids.len());

        // Step 1: Abort the Tokio task
        self.abort_handle.abort();

        // Step 2: Wait a moment for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Step 3: Force kill all tracked Tor daemon PIDs
        for pid in &self.tor_pids {
            info!("🔪 Killing Tor daemon PID: {}", pid);
            if let Err(e) = kill_process_by_pid(*pid).await {
                warn!("⚠️ Failed to kill Tor daemon PID {}: {}", pid, e);
            }
        }

        info!("✅ {} process stopped and {} Tor daemon(s) killed", self.mode, self.tor_pids.len());
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

    /// Get the control port from torrc file configuration
    pub fn get_control_port(&self, path_config: &PathConfig) -> String {
        // Get the appropriate torrc file for this mode
        let torrc_file = self.get_torrc_file();
        let torrc_path = path_config.get_torrc_path(Some(torrc_file));

        // Read the control port from the torrc file
        let control_ports = torrc_parser::get_torrc_config(&torrc_path, "ControlPort");

        if let Some(control_port) = control_ports.first() {
            // Parse port from config value (handles formats like "9992" or "127.0.0.1:9992")
            if let Some(port_num) = torrc_parser::parse_port_from_config(control_port) {
                return port_num.to_string();
            }
            // If parsing fails, return the original value
            control_port.clone()
        } else {
            // Fallback to hardcoded defaults if not found in torrc
            match self {
                EltorMode::Client => "9992".to_string(),
                EltorMode::Relay | EltorMode::Both => "7781".to_string(),
            }
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
                    // Stop the task and kill the Tor processes
                    handle.stop().await?;
                    Ok("Eltor client deactivated".to_string())
                } else {
                    // No handle stored, but try to clean up any orphaned processes anyway
                    info!("⚠️ No client handle found, attempting cleanup of orphaned processes");
                    self.cleanup_orphaned_processes(EltorMode::Client).await;
                    Ok("Eltor client cleaned up (was not tracked)".to_string())
                }
            }
            EltorMode::Relay => {
                let mut relay_guard = self.relay_process.write().await;
                if let Some(mut handle) = relay_guard.take() {
                    // Stop the task and kill the Tor processes
                    handle.stop().await?;
                    Ok("Eltor relay deactivated".to_string())
                } else {
                    // No handle stored, but try to clean up any orphaned processes anyway
                    info!("⚠️ No relay handle found, attempting cleanup of orphaned processes");
                    self.cleanup_orphaned_processes(EltorMode::Relay).await;
                    Ok("Eltor relay cleaned up (was not tracked)".to_string())
                }
            }
            EltorMode::Both => {
                // For "both" mode, we only have one process in the relay slot
                let mut relay_guard = self.relay_process.write().await;
                if let Some(mut handle) = relay_guard.take() {
                    // Stop the task and kill the Tor processes
                    handle.stop().await?;
                    Ok("Eltor both mode deactivated".to_string())
                } else {
                    // No handle stored, but try to clean up any orphaned processes anyway
                    info!("⚠️ No both mode handle found, attempting cleanup of orphaned processes");
                    self.cleanup_orphaned_processes(EltorMode::Both).await;
                    Ok("Eltor both mode cleaned up (was not tracked)".to_string())
                }
            }
        }
    }

    /// Start a new eltor library process as Tokio task
    async fn start_eltor_process(&self, mode: EltorMode) -> Result<EltorProcessHandle, String> {
        let torrc_file = mode.get_torrc_file().to_string();
        let torrc_path = self.path_config.get_torrc_path(Some(&torrc_file));
        let control_port = mode.get_control_port(&self.path_config);

        info!(
            "🚀 Starting eltor {} as Tokio task with torrc: {:?}, control port: {}",
            mode, torrc_path, control_port
        );

        // Clean up any residual state first
        info!("🧹 Pre-start cleanup for {} mode", mode);
        self.cleanup_tor_port(mode.clone()).await;

        let mode_str = mode.to_string().to_string();
        let torrc_path_str = torrc_path.to_string_lossy().to_string();
        let app_data_dir = self.path_config.app_data_dir.clone();
        let control_password = get_tor_control_password(&mode);

        // Spawn the eltor library as an abortable Tokio task
        let task_handle = tokio::spawn(async move {
            let args = vec![
                "eltord".to_string(),
                mode_str.clone(),
                "-f".to_string(),
                torrc_path_str.clone(),
                "-pw".to_string(),
                control_password,
            ];

            info!(
                "🚀 Task starting eltor library {} with args: {:?}",
                mode_str, args
            );

            // Set working directory to app data directory to ensure eltor library can write files
            if let Some(data_dir) = app_data_dir.as_ref() {
                if let Err(e) = std::env::set_current_dir(data_dir) {
                    warn!("⚠️ Failed to set working directory for eltor task: {}", e);
                    warn!("   Eltor library may fail to write files in DMG context");
                }
            }

            // Run eltor library - this will block until shutdown or abort
            eltor::run_with_args(args).await;

            info!("✅ Eltor {} library completed", mode_str);
        });

        // Get the abort handle
        let abort_handle = task_handle.abort_handle();

        // Brief delay to let Tor daemons start
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Discover the PIDs of spawned Tor daemons by checking the control ports
        let mut tor_pids = Vec::new();
        let control_port = mode.get_control_port(&self.path_config);
        
        info!("🔍 Looking for Tor daemon on control port {}", control_port);
        if let Ok(port_num) = control_port.parse::<u16>() {
            if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num) {
                info!("✅ Found Tor daemon PID {} on port {}", pid, control_port);
                tor_pids.push(pid);
            } else {
                warn!("⚠️ No process found on control port {} yet", control_port);
            }
        }

        // For "both" mode, also check the client control port
        if mode == EltorMode::Both {
            let client_port = EltorMode::Client.get_control_port(&self.path_config);
            info!("🔍 Looking for client Tor daemon on control port {}", client_port);
            if let Ok(port_num) = client_port.parse::<u16>() {
                if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num) {
                    info!("✅ Found client Tor daemon PID {} on port {}", pid, client_port);
                    tor_pids.push(pid);
                } else {
                    warn!("⚠️ No process found on client control port {} yet", client_port);
                }
            }
        }

        info!("📋 Tracking {} Tor daemon PID(s) for {} mode", tor_pids.len(), mode);

        Ok(EltorProcessHandle {
            task_handle,
            abort_handle,
            mode,
            tor_pids,
        })
    }

    /// Cleanup Tor daemon for specific mode - MINIMAL and SAFE cleanup
    async fn cleanup_tor_port(&self, mode: EltorMode) {
        let port = mode.get_control_port(&self.path_config);

        info!("🧹 Minimal cleanup for {} mode", mode);

        // ONLY try Tor control commands - NO process killing!
        if let Err(e) = self.send_tor_shutdown_on_port(&port).await {
            warn!(
                "⚠️ Failed to send Tor shutdown command on port {}: {}",
                port, e
            );
        }

        // For Both mode, also try cleaning up client port
        if mode == EltorMode::Both {
            let client_port = EltorMode::Client.get_control_port(&self.path_config);
            if let Err(e) = self.send_tor_shutdown_on_port(&client_port).await {
                warn!(
                    "⚠️ Failed to send Tor shutdown command on client port {}: {}",
                    client_port, e
                );
            }
        }

        // Wait a moment for Tor to process shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        info!("✅ Minimal cleanup completed for {} mode", mode);
    }

    /// Cleanup orphaned processes when no handle is tracked
    /// This is called when deactivate is called but no process handle exists
    async fn cleanup_orphaned_processes(&self, mode: EltorMode) {
        info!("🧹 Cleaning up orphaned processes for {} mode", mode);

        // Find PIDs using the control ports
        let mut orphaned_pids = Vec::new();
        
        let port = mode.get_control_port(&self.path_config);
        info!("🔍 Checking for orphaned process on control port {}", port);
        
        if let Ok(port_num) = port.parse::<u16>() {
            if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num) {
                info!("✅ Found orphaned Tor daemon PID {} on port {}", pid, port);
                orphaned_pids.push(pid);
            } else {
                info!("✓ No orphaned process found on port {}", port);
            }
        }

        // For Both mode, also check client port
        if mode == EltorMode::Both {
            let client_port = EltorMode::Client.get_control_port(&self.path_config);
            info!("🔍 Checking for orphaned client process on port {}", client_port);
            
            if let Ok(port_num) = client_port.parse::<u16>() {
                if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num) {
                    info!("✅ Found orphaned client Tor daemon PID {} on port {}", pid, client_port);
                    orphaned_pids.push(pid);
                } else {
                    info!("✓ No orphaned client process found on port {}", client_port);
                }
            }
        }

        // Kill all found orphaned processes
        if orphaned_pids.is_empty() {
            info!("✓ No orphaned processes to clean up for {} mode", mode);
        } else {
            info!("🔪 Killing {} orphaned process(es) for {} mode", orphaned_pids.len(), mode);
            for pid in orphaned_pids {
                if let Err(e) = kill_process_by_pid(pid).await {
                    warn!("⚠️ Failed to kill orphaned PID {}: {}", pid, e);
                } else {
                    info!("✅ Killed orphaned PID {}", pid);
                }
            }
        }

        info!("✅ Orphaned process cleanup completed for {} mode", mode);
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

        let mut stream = match tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            TcpStream::connect(format!("127.0.0.1:{}", port)),
        )
        .await
        {
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

        // Determine the mode based on the port to get the correct password
        // We need to check which mode uses this port
        let client_port = EltorMode::Client.get_control_port(&self.path_config);
        let relay_port = EltorMode::Relay.get_control_port(&self.path_config);

        let mode = if port == client_port {
            EltorMode::Client
        } else if port == relay_port {
            EltorMode::Relay
        } else {
            // Default to relay mode if we can't determine
            EltorMode::Relay
        };

        let password = get_tor_control_password(&mode);

        // Authenticate
        let auth_command = format!("AUTHENTICATE \"{}\"\r\n", password);
        stream.write_all(auth_command.as_bytes()).await?;

        let mut buf = vec![0; 1024];
        let n = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.read(&mut buf))
            .await??;
        let response = String::from_utf8_lossy(&buf[..n]);

        if response.contains("250 OK") {
            let shutdown_command = "SIGNAL SHUTDOWN\r\n";
            stream.write_all(shutdown_command.as_bytes()).await?;
            info!("🛑 Sent shutdown command to Tor on port {}", port);

            // Give Tor a moment to process the shutdown command
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        } else {
            warn!(
                "⚠️ Failed to authenticate with Tor control port {}: {}",
                port,
                response.trim()
            );
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
        let both_mode_running = relay_guard
            .as_ref()
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

/// Helper function to kill a process by PID
async fn kill_process_by_pid(pid: u32) -> Result<(), String> {
    use crate::ports::kill_process;

    info!("🔪 Killing process PID {}", pid);
    kill_process(pid)?;
    
    // Wait a moment to let the process die
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    
    Ok(())
}
