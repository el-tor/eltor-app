use log::{info, warn};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::paths::{is_tauri_context, PathConfig};
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

        // Step 1: Stop Arti if this is the last eltord process stopping
        info!("🛑 Stopping Arti process...");
        if let Err(e) = crate::arti::stop_arti().await {
            warn!("⚠️ Failed to stop Arti: {}", e);
        } else {
            info!("✅ Arti stopped successfully");
        }

        // Step 2: Abort the Tokio task
        self.abort_handle.abort();

        // Step 3: Wait a moment for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Step 4: Force kill all tracked Tor daemon PIDs
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
}

/// Get the PID file path for a given mode and path configuration
/// 
/// This centralizes the logic for determining where PID files are stored:
/// - Tauri mode: Uses app_data_dir (e.g., ~/Library/Application Support/eltor/)
/// - Web mode: Uses bin_dir/data/
fn get_pid_file_path(mode: &EltorMode, path_config: &PathConfig) -> std::path::PathBuf {
    let path = if let Some(app_data_dir) = &path_config.app_data_dir {
        // Tauri mode - PID files in app data directory
        match mode {
            EltorMode::Client => app_data_dir.join("eltord-client.pid"),
            EltorMode::Relay | EltorMode::Both => app_data_dir.join("eltord-relay.pid"),
        }
    } else {
        // Non-Tauri mode - PID files in bin/data
        match mode {
            EltorMode::Client => path_config.bin_dir.join("data").join("eltord-client.pid"),
            EltorMode::Relay | EltorMode::Both => path_config.bin_dir.join("data").join("eltord-relay.pid"),
        }
    };
    // eprintln!("🔧 [get_pid_file_path] mode={:?}, is_tauri={}, path={:?}", mode, path_config.app_data_dir.is_some(), path);
    path
}

/// Clean up old data files before activation
fn cleanup_old_data_files(mode: &EltorMode, path_config: &PathConfig) {
    use std::fs;
    
    log::info!("🧹 Cleaning up old data files for mode: {:?}", mode);
    
    // Determine base data directory
    let data_dir = if let Some(app_data_dir) = &path_config.app_data_dir {
        // Tauri mode
        app_data_dir.clone()
    } else {
        // Non-Tauri mode
        path_config.bin_dir.join("data")
    };
    
    // Always clean eltor.log, regardless of mode
    let log_file = data_dir.join("eltor.log");
    if log_file.exists() {
        match fs::remove_file(&log_file) {
            Ok(_) => log::info!("🧹 Deleted: {:?}", log_file),
            Err(e) => log::warn!("⚠️ Failed to delete {:?}: {}", log_file, e),
        }
    }
    
    // Always clean payment files, regardless of mode
    let payments_sent = data_dir.join("payments_sent.json");
    if payments_sent.exists() {
        match fs::remove_file(&payments_sent) {
            Ok(_) => log::info!("🧹 Deleted: {:?}", payments_sent),
            Err(e) => log::warn!("⚠️ Failed to delete {:?}: {}", payments_sent, e),
        }
    }
    
    let payments_received = data_dir.join("payments_received.json");
    if payments_received.exists() {
        match fs::remove_file(&payments_received) {
            Ok(_) => log::info!("🧹 Deleted: {:?}", payments_received),
            Err(e) => log::warn!("⚠️ Failed to delete {:?}: {}", payments_received, e),
        }
    }
    
    // Define Tor cache files to delete
    let tor_cache_files = [
        "cached-consensus",
        "cached-certs",
        "cached-extrainfo",
        "cached-extrainfo.new",
        "cached-consensus.new",
        "cached-descriptors",
        "cached-descriptors.new",
        "cached-microdesc-consensus",
        "cached-microdesc-consensus.new",
        "cached-microdescs",
        "cached-microdescs.new",
        "my-consensus-microdesc",
        "my-consensus-ns",
        "router-stability",
        "state",
        "sr-state",
        "unverified-consensus",
        "v3-status-votes",
    ];
    
    // Always clean Tor log files and cache files for both client and relay, regardless of mode
    for subdir in ["client", "relay"] {
        let tor_data_path = data_dir.join("tor_data").join(subdir);
        
        // Delete log files
        for log_name in ["debug.log", "info.log", "notice.log"] {
            let log_path = tor_data_path.join(log_name);
            if log_path.exists() {
                match fs::remove_file(&log_path) {
                    Ok(_) => log::info!("🧹 Deleted: {:?}", log_path),
                    Err(e) => log::warn!("⚠️ Failed to delete {:?}: {}", log_path, e),
                }
            }
        }
        
        // Delete cache files
        for cache_file in &tor_cache_files {
            let cache_path = tor_data_path.join(cache_file);
            if cache_path.exists() {
                match fs::remove_file(&cache_path) {
                    Ok(_) => log::info!("🧹 Deleted: {:?}", cache_path),
                    Err(e) => log::warn!("⚠️ Failed to delete {:?}: {}", cache_path, e),
                }
            }
        }
    }
    
    log::info!("✅ Cleanup completed");
}

impl EltorMode {
    /// Get the control port from torrc file configuration
    pub async fn get_control_port(&self, path_config: &PathConfig) -> String {
        // Get the appropriate torrc file for this mode
        let torrc_file = self.get_torrc_file();
        let torrc_path = path_config.get_torrc_path(Some(torrc_file));

        // Read the control port from the torrc file
        let control_ports = torrc_parser::get_torrc_config(&torrc_path, "ControlPort").await;

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
/// TODO depreciate and remove EltorManager in future release
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
        let _control_port = mode.get_control_port(&self.path_config);

        // Start Arti alongside eltord
        info!("🚀 Starting Arti for eltord mode: {}", mode);
        if let Err(e) = crate::arti::start_arti_with_eltord(mode.to_string(), &self.path_config).await {
            warn!("⚠️ Failed to start Arti: {}", e);
            warn!("   Continuing with eltord startup...");
        } else {
            info!("✅ Arti started successfully for mode: {}", mode);
        }

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
                "-p".to_string(),
                control_password,
                "-k".to_string(),
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

            // Turn off logs to avoid blocking eltord binary output (since we log in eltord with -l option)
            log::set_max_level(log::LevelFilter::Off);

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
        let control_port = mode.get_control_port(&self.path_config).await;
        
        // info!("🔍 Looking for Tor daemon on control port {}", control_port);
        if let Ok(port_num) = control_port.parse::<u16>() {
            if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num).await {
                info!("✅ Found Tor daemon PID {} on port {}", pid, control_port);
                tor_pids.push(pid);
            } else {
                warn!("⚠️ No process found on control port {} yet", control_port);
            }
        }

        // For "both" mode, also check the client control port
        if mode == EltorMode::Both {
            let client_port = EltorMode::Client.get_control_port(&self.path_config).await;
            info!("🔍 Looking for client Tor daemon on control port {}", client_port);
            if let Ok(port_num) = client_port.parse::<u16>() {
                if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num).await {
                    info!("✅ Found client Tor daemon PID {} on port {}", pid, client_port);
                    tor_pids.push(pid);
                } else {
                    warn!("⚠️ No process found on client control port {} yet", client_port);
                }
            }
        }

        info!("📋 Tracking {} Tor daemon PID(s) for {} mode", tor_pids.len(), &mode);

        Ok(EltorProcessHandle {
            task_handle,
            abort_handle,
            mode: mode.clone(),
            tor_pids,
        })
    }

    /// Cleanup Tor daemon for specific mode - MINIMAL and SAFE cleanup
    async fn cleanup_tor_port(&self, mode: EltorMode) {
        let port = mode.get_control_port(&self.path_config).await;

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
            let client_port = EltorMode::Client.get_control_port(&self.path_config).await;
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
        
        let port = mode.get_control_port(&self.path_config).await;
        info!("🔍 Checking for orphaned process on control port {}", port);
        
        if let Ok(port_num) = port.parse::<u16>() {
            if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num).await {
                info!("✅ Found orphaned Tor daemon PID {} on port {}", pid, port);
                orphaned_pids.push(pid);
            } else {
                info!("✓ No orphaned process found on port {}", port);
            }
        }

        // For Both mode, also check client port
        if mode == EltorMode::Both {
            let client_port = EltorMode::Client.get_control_port(&self.path_config).await;
            info!("🔍 Checking for orphaned client process on port {}", client_port);
            
            if let Ok(port_num) = client_port.parse::<u16>() {
                if let Ok(Some(pid)) = crate::ports::get_pid_using_port(port_num).await {
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
        let client_port = EltorMode::Client.get_control_port(&self.path_config).await;
        let relay_port = EltorMode::Relay.get_control_port(&self.path_config).await;

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

/// Check if eltord is running by reading PID file and verifying process exists
pub async fn is_eltord_running(mode: EltorMode, path_config: &PathConfig) -> bool {
    // eprintln!("🔍 [is_eltord_running] Checking mode={:?}, is_tauri={}", mode, path_config.app_data_dir.is_some());
    let pid_file = get_pid_file_path(&mode, path_config);
    log::info!("🔍 [is_eltord_running] Checking PID file: {:?} (Tauri: {})", pid_file, path_config.app_data_dir.is_some());

    // Read PID from file
    let pid = match tokio::fs::read_to_string(&pid_file).await {
        Ok(content) => match content.trim().parse::<u32>() {
            Ok(pid) => {
                // eprintln!("✅ [is_eltord_running] Found PID {} in file {:?}", pid, pid_file);
                pid
            },
            Err(_) => {
                // eprintln!("❌ [is_eltord_running] Invalid PID in file {:?}", pid_file);
                return false;
            }
        },
        Err(_e) => {
            // eprintln!("❌ [is_eltord_running] No PID file at {:?}: {}", pid_file, e);
            return false;
        }
    };

    // Verify process is actually running
    #[cfg(target_os = "macos")]
    {
        use std::process::Command as StdCommand;
        match StdCommand::new("kill")
            .arg("-0") // Signal 0 just checks if process exists
            .arg(pid.to_string())
            .output()
        {
            Ok(output) => {
                let is_running = output.status.success();
                // eprintln!("🔍 [is_eltord_running] Process {} exists: {}", pid, is_running);
                is_running
            },
            Err(_e) => {
                // eprintln!("❌ [is_eltord_running] Failed to check process {}: {}", pid, e);
                false
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Check if /proc/<pid> exists
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // Fallback - assume running if PID file exists
        true
    }
}

/// Get eltord status by checking PID files
pub async fn get_eltord_status_from_pid_files(path_config: &PathConfig) -> EltorStatus {
    let client_running = is_eltord_running(EltorMode::Client, path_config).await;
    let relay_running = is_eltord_running(EltorMode::Relay, path_config).await;

    EltorStatus {
        running: client_running || relay_running,
        client_running,
        relay_running,
        recent_logs: vec![], // No logs tracking in this simple approach
    }
}

/// Helper function to send Tor shutdown command to a control port
async fn send_tor_shutdown_command(
    port: &str,
    mode: &EltorMode,
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

    let password = get_tor_control_password(mode);

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

/// Helper function to check if a process with given PID is still running
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command as StdCommand;
        match StdCommand::new("kill")
            .arg("-0") // Signal 0 just checks if process exists
            .arg(pid.to_string())
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // Fallback - we can't reliably check, so assume it's running
        true
    }
}

/// Deactivate eltord by reading PID file and attempting graceful shutdown first
pub async fn deactivate_eltord_process(mode: String) -> Result<String, String> {
    // eprintln!("🛑 [deactivate_eltord_process] Called with mode={}", mode);
    let mode_enum = match EltorMode::from_str(&mode) {
        Ok(m) => m,
        Err(_) => {
            return Err(format!("Invalid eltor mode: {}", mode));
        }
    };

    // Get path config
    let path_config = match crate::paths::PathConfig::new() {
        Ok(pc) => {
            // eprintln!("🛑 [deactivate_eltord_process] PathConfig: is_tauri={}", pc.app_data_dir.is_some());
            pc
        },
        Err(e) => {
            return Err(format!("Failed to get path config: {}", e));
        }
    };
    
    let pid_file = get_pid_file_path(&mode_enum, &path_config);
    log::info!("🛑 [deactivate_eltord_process] Looking for PID file: {:?} (Tauri: {})", pid_file, path_config.app_data_dir.is_some());

    // Read PID from file
    let pid = match std::fs::read_to_string(&pid_file) {
        Ok(content) => match content.trim().parse::<u32>() {
            Ok(pid) => {
                // eprintln!("✅ [deactivate_eltord_process] Found PID {} in {:?}", pid, pid_file);
                pid
            },
            Err(_) => {
                // eprintln!("❌ [deactivate_eltord_process] Invalid PID in file {:?}", pid_file);
                return Err(format!("Invalid PID in file {:?}", pid_file));
            }
        },
        Err(_e) => {
            // eprintln!("❌ [deactivate_eltord_process] No PID file found at {:?}: {}", pid_file, e);
            return Err(format!("No PID file found at {:?} - process may not be running", pid_file));
        }
    };

    log::info!("🛑 Attempting graceful shutdown of eltord {} (PID: {})", mode_enum, pid);

    // Step 1: Attempt graceful shutdown via Tor control port
    let control_port = mode_enum.get_control_port(&path_config).await;
    
    // Try to send shutdown command - log errors but don't fail
    if let Err(e) = send_tor_shutdown_command(&control_port, &mode_enum).await {
        log::warn!("⚠️ Failed to send graceful shutdown to Tor control port {}: {}", control_port, e);
        log::info!("   Will attempt forceful shutdown as fallback");
    } else {
        log::info!("✅ Graceful shutdown command sent, waiting for process to exit...");
        
        // Step 2: Poll for process exit with timeout (total ~8 seconds)
        const POLL_INTERVAL_MS: u64 = 200;
        const MAX_POLLS: u32 = 40; // 40 * 200ms = 8 seconds
        
        for poll_count in 0..MAX_POLLS {
            tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
            
            if !is_process_running(pid) {
                log::info!("✅ Process {} exited gracefully after {}ms", 
                    pid, poll_count * POLL_INTERVAL_MS as u32);
                
                // Process has exited - skip kill_process and just clean up PID file
                if let Err(e) = std::fs::remove_file(&pid_file) {
                    // eprintln!("⚠️ [deactivate_eltord_process] Failed to remove PID file {:?}: {}", pid_file, e);
                    log::warn!("⚠️ Failed to remove PID file {:?}: {}", pid_file, e);
                } else {
                    // eprintln!("✅ [deactivate_eltord_process] Removed PID file: {:?}", pid_file);
                    log::info!("🗑️ Removed PID file: {:?}", pid_file);
                }
                
                log::info!("✅ Eltord {} stopped gracefully (PID: {})", mode_enum, pid);
                // eprintln!("✅ [deactivate_eltord_process] Successfully deactivated {} (PID: {})", mode_enum, pid);
                
                // Stop SOCKS router after successful eltord deactivation
                log::info!("🔀 Stopping SOCKS Router...");
                if let Err(e) = crate::socks::stop_socks_router().await {
                    log::warn!("⚠️ Failed to stop SOCKS Router: {}", e);
                } else {
                    log::info!("✅ SOCKS Router stopped successfully");
                }
                
                // Stop Arti after successful eltord deactivation
                log::info!("🛑 Stopping Arti...");
                if let Err(e) = crate::arti::stop_arti().await {
                    log::warn!("⚠️ Failed to stop Arti: {}", e);
                } else {
                    log::info!("✅ Arti stopped successfully");
                }
                
                return Ok(format!("Eltord {} deactivated gracefully", mode_enum));
            }
        }
        
        log::warn!("⚠️ Process {} did not exit after {}s, will force kill", 
            pid, (MAX_POLLS * POLL_INTERVAL_MS as u32) / 1000);
    }

    // Step 3: Fallback to forceful kill if graceful shutdown failed or timed out
    log::info!("🔪 Force killing eltord {} (PID: {})", mode_enum, pid);
    use crate::ports::kill_process;
    if let Err(e) = kill_process(pid) {
        // eprintln!("❌ [deactivate_eltord_process] Failed to kill process {}: {}", pid, e);
        log::error!("❌ Failed to force kill process {}: {}", pid, e);
        return Err(format!("Failed to kill process {}: {}", pid, e));
    }
    // eprintln!("✅ [deactivate_eltord_process] Killed process {}", pid);

    // Remove PID file
    if let Err(e) = std::fs::remove_file(&pid_file) {
        // eprintln!("⚠️ [deactivate_eltord_process] Failed to remove PID file {:?}: {}", pid_file, e);
        log::warn!("⚠️ Failed to remove PID file {:?}: {}", pid_file, e);
    } else {
        // eprintln!("✅ [deactivate_eltord_process] Removed PID file: {:?}", pid_file);
        log::info!("🗑️ Removed PID file: {:?}", pid_file);
    }

    log::info!("✅ Eltord {} stopped (force killed, PID: {})", mode_enum, pid);
    // eprintln!("✅ [deactivate_eltord_process] Successfully deactivated {} (PID: {})", mode_enum, pid);
    
    // Stop SOCKS router after successful eltord deactivation
    log::info!("🔀 Stopping SOCKS Router...");
    if let Err(e) = crate::socks::stop_socks_router().await {
        log::warn!("⚠️ Failed to stop SOCKS Router: {}", e);
    } else {
        log::info!("✅ SOCKS Router stopped successfully");
    }
    
    // Stop Arti after successful eltord deactivation
    log::info!("🛑 Stopping Arti...");
    if let Err(e) = crate::arti::stop_arti().await {
        log::warn!("⚠️ Failed to stop Arti: {}", e);
    } else {
        log::info!("✅ Arti stopped successfully");
    }
    
    Ok(format!("Eltord {} deactivated (force killed)", mode_enum))
}

/// Cleanup all eltord processes across all modes
/// This is useful for shutdown handlers in both Tauri and Axum
pub async fn cleanup_all_eltord_processes() {
    log::info!("🧹 Cleaning up all eltord processes...");
    
    for mode in &["client", "relay", "both"] {
        match deactivate_eltord_process(mode.to_string()).await {
            Ok(msg) => log::info!("✅ {}", msg),
            Err(e) => {
                // Only warn if it's not a "no PID file" error
                if !e.contains("No PID file found") && !e.contains("not running") {
                    log::warn!("⚠️ {}", e);
                }
            }
        }
    }
    
    log::info!("✅ All eltord processes cleaned up");
}

// TODO clean this up
pub fn activate_eltord_process(mode: String, enable_logging: bool) {
    // eprintln!("🚀 [activate_eltord_process] Called with mode={}, enable_logging={}", mode, enable_logging);
    log::info!("🚀 [activate_eltord_process] mode={}, enable_logging={}", mode, enable_logging);

    let mode_enum = match EltorMode::from_str(&mode) {
        Ok(m) => m,
        Err(_) => {
            warn!("⚠️ Invalid eltor mode specified for activation: {}", mode);
            return;
        }
    };

    // Create strings early to avoid lifetime issues  
    let mode_str_for_arti = mode.to_string(); // Use the original mode string instead
    let mode_str_for_logging = mode.to_string();

    // Get path config based on context (Tauri vs web)
    let path_config = if is_tauri_context() {
        // eprintln!("🚀 [activate_eltord_process] Running in Tauri mode");
        // In Tauri, use app data directory
        let app_data_dir = match dirs::data_dir() {
            Some(dir) => dir.join("eltor"),
            None => {
                warn!("⚠️ Failed to get app data directory");
                return;
            }
        };
        
        // Try to create app data directory
        if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
            warn!("⚠️ Failed to create app data directory: {}", e);
            return;
        }
        
        // In Tauri mode, bin_dir should come from environment variable set by Tauri frontend
        // This allows Tauri to pass the resource directory path
        let bin_dir = if let Ok(tauri_bin_dir) = env::var("ELTOR_TAURI_BIN_DIR") {
            let bin_path = std::path::PathBuf::from(tauri_bin_dir);
            // eprintln!("🚀 [activate_eltord_process] Using ELTOR_TAURI_BIN_DIR: {:?}", bin_path);
            bin_path
        } else {
            // Fallback for development mode
            match env::current_dir() {
                Ok(cwd) => {
                    let dev_bin = cwd.join("../../backend/bin");
                    if dev_bin.exists() {
                        // eprintln!("🚀 [activate_eltord_process] Using development bin dir: {:?}", dev_bin);
                        dev_bin
                    } else {
                        warn!("⚠️ ELTOR_TAURI_BIN_DIR not set and dev bin not found, using app_data_dir");
                        app_data_dir.clone()
                    }
                }
                Err(_) => {
                    warn!("⚠️ Could not determine bin_dir, using app_data_dir");
                    app_data_dir.clone()
                }
            }
        };
        
        PathConfig {
            bin_dir,
            data_dir: app_data_dir.clone(),
            app_data_dir: Some(app_data_dir),
        }
    } else {
        // eprintln!("🚀 [activate_eltord_process] Running in web mode");
        // Non-Tauri mode - use standard path detection
        match PathConfig::new() {
            Ok(pc) => pc,
            Err(e) => {
                warn!("⚠️ Failed to get path config: {}", e);
                return;
            }
        }
    };
    
    let torrc_file = mode_enum.get_torrc_file();
    let torrc_path = path_config.get_torrc_path(Some(torrc_file));
    let torrc_path_str = torrc_path.to_string_lossy().to_string();
    let control_password = std::env::var(match mode_enum {
        EltorMode::Client => "APP_ELTOR_TOR_CONTROL_PASSWORD",
        EltorMode::Relay | EltorMode::Both => "APP_ELTOR_TOR_RELAY_CONTROL_PASSWORD",
    })
    .unwrap_or_else(|_| "password1234_".to_string());
    let eltord_path = path_config.bin_dir.join("eltord");
    // Determine correct log path based on context
    let eltord_log_path = if let Some(app_data_dir) = path_config.app_data_dir.as_ref() {
        // Tauri mode - use app data directory
        app_data_dir.join("eltor.log")
    } else {
        // Non-Tauri mode - use bin/data directory
        path_config.bin_dir.join("data").join("eltor.log")
    };
    
    let pid_file = get_pid_file_path(&mode_enum, &path_config);
    log::info!("🚀 [activate_eltord_process] Will write PID file to: {:?} (Tauri: {})", pid_file, path_config.app_data_dir.is_some());

    // if is_tauri_context() {
    //     eprintln!("isTauriContext=true, {:?}", path_config);
    // } else {
    //     eprintln!("isTauriContext=false, {:?}", path_config);
    // }

    let log_path_str = eltord_log_path.to_str().unwrap_or_default().to_string();

    // Clean up old data files before activation
    cleanup_old_data_files(&mode_enum, &path_config);

    // Start Arti before starting eltord (synchronously to avoid lifetime issues)
    info!("🚀 Starting Arti for eltord mode: {}", mode_str_for_logging);
    // Note: We'll start Arti in a simple, blocking way here since it's fast
    // The actual work will be done by the spawned eltord process
    
    log::info!("🚀 Spawning eltord {} with torrc: {:?}", mode_enum, torrc_path);
    log::info!("   Torrc path string: {}", torrc_path_str);
    log::info!("   Log path: {:?}", eltord_log_path);
    log::info!("   Log path string: {}", log_path_str);
    log::info!("   PID file: {:?}", pid_file);
    log::info!("   Working dir: {:?}", path_config.bin_dir);

    // Use std::process::Command for true isolation - NO tokio involvement
    use std::process::Command as StdCommand;
    
    #[cfg(target_os = "macos")]
    {
        // macOS-specific: Use posix_spawn to avoid fork() issues in multi-threaded environments
        // Tauri apps create multiple threads (WebView, UI, etc.), and fork() after threading
        // causes crashes with "multi-threaded process forked" errors
        // By not using .pre_exec(), std::process::Command uses posix_spawn instead of fork+exec
        
        log::info!("🚀 [macOS] Attempting to spawn eltord:");
        log::info!("   Binary: {:?}", eltord_path);
        log::info!("   Exists: {}", eltord_path.exists());
        log::info!("   Working dir: {:?}", path_config.bin_dir);
        log::info!("   Mode: {}", mode_enum);
        log::info!("   Torrc: {}", torrc_path_str);
        log::info!("   Log file: {}", log_path_str);
        log::info!("   PID file: {:?}", pid_file);
        
        // Check if binary exists and is executable
        if !eltord_path.exists() {
            let error_msg = format!("❌ eltord binary not found at {:?}", eltord_path);
            // eprintln!("{}", error_msg);
            log::error!("{}", error_msg);
            return;
        }
        
        let mut cmd = StdCommand::new(&eltord_path);
        cmd.arg(mode_enum.to_string())
            .arg("-f")
            .arg(&torrc_path_str)
            .arg("-p")
            .arg(&control_password);
        
        // Conditionally add logging arguments if enabled
        if enable_logging {
            cmd.arg("-l")
                .arg(&log_path_str)
                .arg("-k");
        }
        
        cmd.current_dir(&path_config.bin_dir);
            //.stdout(Stdio::null())
            //.stderr(Stdio::null())
            //.stdin(Stdio::null())
            // On macOS, prefer posix_spawn over fork (avoid multi-thread fork issues)
            // This is critical for Tauri apps which have multiple threads running
        
        match cmd.spawn()
        {
            Ok(child) => {
                let pid = child.id();
                log::info!("✅ Eltord {} spawned with PID: {} - process is now independent", mode_enum, pid);
                log::info!("⏳ Tor will bootstrap in background (10-15 seconds typical)");
                
                // Start Arti after eltord successfully starts
                log::info!("🚀 Starting Arti for eltord mode: {}", mode_str_for_logging);
                let mode_for_arti = mode_str_for_arti.clone();
                let path_for_arti = path_config.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = crate::arti::start_arti_with_eltord(&mode_for_arti, &path_for_arti).await {
                            warn!("⚠️ Failed to start Arti: {}", e);
                        } else {
                            info!("✅ Arti started successfully for mode: {}", mode_for_arti);
                        }
                        
                        // Start SOCKS router after Arti is ready
                        info!("🔀 Starting SOCKS Router...");
                        if let Err(e) = crate::socks::start_socks_router().await {
                            warn!("⚠️ SOCKS Router failed to start: {}", e);
                            info!("   This is non-critical - eltord will still function without the SOCKS router");
                        } else {
                            let router_port = std::env::var("APP_ELTOR_SOCKS_ROUTER_PORT")
                                .unwrap_or_else(|_| "18048".to_string());
                            info!("✅ SOCKS Router started successfully on port {}", router_port);
                        }
                    });
                });
                
                // Write PID to file synchronously
                if let Err(e) = std::fs::write(&pid_file, pid.to_string()) {
                    // eprintln!("⚠️ [activate_eltord_process] Failed to write PID file {:?}: {}", pid_file, e);
                    log::warn!("⚠️ Failed to write PID file {:?}: {}", pid_file, e);
                } else {
                    // eprintln!("✅ [activate_eltord_process] Wrote PID {} to {:?}", pid, pid_file);
                    log::info!("✅ Wrote PID {} to {:?}", pid, pid_file);
                }
                
                // Process is now 100% isolated - we don't even wait on it
                // It will be reaped by init when it exits
                std::mem::forget(child); // Don't wait, don't reap, just let it run
                
                // eprintln!("✅ [activate_eltord_process] Successfully activated {} (PID: {})", mode_enum, pid);
                log::info!("🎯 Activation complete - eltord is running independently (PID: {})", pid);
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to spawn eltord {}: {}", mode_enum, e);
                // eprintln!("[activate_eltord_process] {}", error_msg);
                log::error!("{}", error_msg);
                log::error!("   Error kind: {:?}", e.kind());
                log::error!("   Binary path: {:?}", eltord_path);
                log::error!("   Working dir: {:?}", path_config.bin_dir);
                
                // Try to get more details about why it failed
                if let Some(os_error) = e.raw_os_error() {
                    log::error!("   OS error code: {}", os_error);
                }
            }
        }
    }
    
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::os::unix::process::CommandExt;
        
        let mut cmd = StdCommand::new(&eltord_path);
        cmd.arg(mode_enum.to_string())
            .arg("-f")
            .arg(&torrc_path_str)
            .arg("-p")
            .arg(&control_password);
        
        // Conditionally add logging arguments if enabled
        if enable_logging {
            cmd.arg("-l")
                .arg(&log_path_str)
                .arg("-k");
        }
        
        unsafe {
            cmd.current_dir(&path_config.bin_dir)
                .pre_exec(|| {
                    // Create new session - completely detach from parent
                    libc::setsid();
                    
                    // Close all file descriptors except stdin/out/err
                    // This prevents inheriting any open sockets or files
                    let max_fd = libc::sysconf(libc::_SC_OPEN_MAX);
                    if max_fd > 0 {
                        for fd in 3..max_fd {
                            libc::close(fd as i32);
                        }
                    }
                    
                    Ok(())
                });
        }
        
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                log::info!("✅ Eltord {} spawned with PID: {} - process is now independent", mode_enum, pid);
                log::info!("⏳ Tor will bootstrap in background (10-15 seconds typical)");
                
                // Start Arti after eltord successfully starts
                log::info!("🚀 Starting Arti for eltord mode: {}", mode_str_for_logging);
                let mode_for_arti = mode_str_for_arti.clone();
                let path_for_arti = path_config.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = crate::arti::start_arti_with_eltord(&mode_for_arti, &path_for_arti).await {
                            warn!("⚠️ Failed to start Arti: {}", e);
                        } else {
                            info!("✅ Arti started successfully for mode: {}", mode_for_arti);
                        }
                        
                        // Start SOCKS router after Arti is ready
                        info!("🔀 Starting SOCKS Router...");
                        if let Err(e) = crate::socks::start_socks_router().await {
                            warn!("⚠️ SOCKS Router failed to start: {}", e);
                            info!("   This is non-critical - eltord will still function without the SOCKS router");
                        } else {
                            let router_port = std::env::var("APP_ELTOR_SOCKS_ROUTER_PORT")
                                .unwrap_or_else(|_| "18048".to_string());
                            info!("✅ SOCKS Router started successfully on port {}", router_port);
                        }
                    });
                });
                
                // Write PID to file synchronously
                if let Err(e) = std::fs::write(&pid_file, pid.to_string()) {
                    // eprintln!("⚠️ [activate_eltord_process] Failed to write PID file {:?}: {}", pid_file, e);
                    log::warn!("⚠️ Failed to write PID file {:?}: {}", pid_file, e);
                } else {
                    // eprintln!("✅ [activate_eltord_process] Wrote PID {} to {:?}", pid, pid_file);
                }
                
                // Process is now 100% isolated - we don't even wait on it
                // It will be reaped by init when it exits
                std::mem::forget(child); // Don't wait, don't reap, just let it run
                
                // eprintln!("✅ [activate_eltord_process] Successfully activated {} (PID: {})", mode_enum, pid);
                log::info!("🎯 Activation complete - eltord is running independently (PID: {})", pid);
            }
            Err(e) => {
                // eprintln!("❌ [activate_eltord_process] Failed to spawn eltord {}: {}", mode_enum, e);
                log::error!("❌ Failed to spawn eltord {}: {}", mode_enum, e);
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        let mut cmd = StdCommand::new(&eltord_path);
        cmd.arg(mode_enum.to_string())
            .arg("-f")
            .arg(&torrc_path_str)
            .arg("-p")
            .arg(&control_password);
        
        // Conditionally add logging arguments if enabled
        if enable_logging {
            cmd.arg("-l")
                .arg(&log_path_str)
                .arg("-k");
        }
        
        cmd.current_dir(&path_config.bin_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        
        match cmd.spawn()
        {
            Ok(child) => {
                let pid = child.id();
                log::info!("✅ Eltord {} spawned with PID: {} - process is now independent", mode_enum, pid);
                log::info!("⏳ Tor will bootstrap in background (10-15 seconds typical)");
                
                // Start Arti after eltord successfully starts
                log::info!("🚀 Starting Arti for eltord mode: {}", mode_str_for_logging);
                let mode_for_arti = mode_str_for_arti.clone();
                let path_for_arti = path_config.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = crate::arti::start_arti_with_eltord(&mode_for_arti, &path_for_arti).await {
                            warn!("⚠️ Failed to start Arti: {}", e);
                        } else {
                            info!("✅ Arti started successfully for mode: {}", mode_for_arti);
                        }
                        
                        // Start SOCKS router after Arti is ready
                        info!("🔀 Starting SOCKS Router...");
                        if let Err(e) = crate::socks::start_socks_router().await {
                            warn!("⚠️ SOCKS Router failed to start: {}", e);
                            info!("   This is non-critical - eltord will still function without the SOCKS router");
                        } else {
                            let router_port = std::env::var("APP_ELTOR_SOCKS_ROUTER_PORT")
                                .unwrap_or_else(|_| "18048".to_string());
                            info!("✅ SOCKS Router started successfully on port {}", router_port);
                        }
                    });
                });
                
                // Write PID to file synchronously
                if let Err(e) = std::fs::write(&pid_file, pid.to_string()) {
                    log::warn!("⚠️ Failed to write PID file {:?}: {}", pid_file, e);
                }
                
                std::mem::forget(child);
                
                log::info!("🎯 Activation complete - eltord is running independently (PID: {})", pid);
            }
            Err(e) => {
                log::error!("❌ Failed to spawn eltord {}: {}", mode_enum, e);
            }
        }
    }
}
