use log::{info, warn, error};
use std::process::{Command as StdCommand, Stdio};
use std::env;

use crate::paths::PathConfig;

/// Get the Arti SOCKS port from environment variables
/// 
/// This function looks for the APP_ARTI_SOCKS_PORT environment variable.
/// If not found, defaults to 18050.
fn get_arti_socks_port() -> u16 {
    env::var("APP_ARTI_SOCKS_PORT")
        .unwrap_or_else(|_| "18050".to_string())
        .parse::<u16>()
        .unwrap_or(18050)
}

/// Arti process handle for tracking the running Arti instance
#[derive(Debug)]
pub struct ArtiProcessHandle {
    pid: u32,
    mode: String, // Store mode for logging purposes
}

impl ArtiProcessHandle {
    pub fn new(pid: u32, mode: String) -> Self {
        Self { pid, mode }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Stop the Arti process
    pub async fn stop(&self) -> Result<(), String> {
        info!("🛑 Stopping Arti process (PID: {})", self.pid);

        // First try graceful shutdown by killing the process
        if let Err(e) = crate::ports::kill_process(self.pid) {
            warn!("⚠️ Failed to kill Arti process {}: {}", self.pid, e);
            return Err(format!("Failed to kill Arti process: {}", e));
        }

        // Wait a moment for the process to exit
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("✅ Arti process stopped (PID: {})", self.pid);
        Ok(())
    }
}

/// Global Arti process handle storage
static ARTI_PROCESS: tokio::sync::RwLock<Option<ArtiProcessHandle>> = tokio::sync::RwLock::const_new(None);

/// Start Arti process when eltord starts
pub async fn start_arti_with_eltord(mode: &str, path_config: &PathConfig) -> Result<(), String> {
    // Check if Arti is already running
    {
        let arti_guard = ARTI_PROCESS.read().await;
        if arti_guard.is_some() {
            info!("ℹ️  Arti is already running, skipping startup");
            return Ok(());
        }
    }

    let socks_port = get_arti_socks_port();
    info!("🚀 Starting Arti for eltord mode: {} (SOCKS port: {})", mode, socks_port);

    let arti_binary = path_config.get_executable_path("arti");
    
    // Check if Arti binary exists
    if !arti_binary.exists() {
        let error_msg = format!("❌ Arti binary not found at {:?}", arti_binary);
        error!("{}", error_msg);
        return Err(error_msg);
    }

    // Start Arti process
    let socks_config = format!("proxy.socks_port={}", socks_port);
    let mut cmd = StdCommand::new(&arti_binary);
    cmd.arg("proxy")
        .arg("-o")
        .arg(&socks_config)
        .current_dir(&path_config.bin_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    info!("🚀 Starting Arti with SOCKS port {}", socks_port);
    info!("   Binary: {:?}", arti_binary);
    info!("   Working dir: {:?}", path_config.bin_dir);

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            info!("✅ Arti started with PID: {} for mode: {}", pid, mode);
            info!("   SOCKS proxy available on port {}", socks_port);

            // Wait a moment to see if the process stays alive
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            
            // Check if process is still running
            if !is_process_running(pid) {
                let error_msg = format!("❌ Arti process {} died immediately after start", pid);
                error!("{}", error_msg);
                return Err(error_msg);
            }

            // Store the process handle
            {
                let mut arti_guard = ARTI_PROCESS.write().await;
                *arti_guard = Some(ArtiProcessHandle::new(pid, mode.to_string()));
            }

            // Let the process run independently
            std::mem::forget(child);
            
            info!("🎯 Arti startup completed successfully (PID: {}, SOCKS port: {})", pid, socks_port);
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("❌ Failed to start Arti: {}", e);
            error!("{}", error_msg);
            error!("   Check if Arti binary is executable");
            Err(error_msg)
        }
    }
}

/// Stop Arti process
pub async fn stop_arti() -> Result<(), String> {
    let mut arti_guard = ARTI_PROCESS.write().await;
    
    if let Some(handle) = arti_guard.take() {
        handle.stop().await?;
        info!("🛑 Arti process stopped and handle removed");
        Ok(())
    } else {
        info!("ℹ️  No Arti process to stop");
        Ok(())
    }
}

/// Check if Arti is currently running
pub async fn is_arti_running() -> bool {
    let arti_guard = ARTI_PROCESS.read().await;
    
    if let Some(handle) = arti_guard.as_ref() {
        // Verify the process is actually still running
        is_process_running(handle.pid())
    } else {
        false
    }
}

/// Get Arti process status
pub async fn get_arti_status() -> Option<(u32, String)> {
    let arti_guard = ARTI_PROCESS.read().await;
    
    if let Some(handle) = arti_guard.as_ref() {
        if is_process_running(handle.pid()) {
            Some((handle.pid(), handle.mode.clone()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Helper function to check if a process is running
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
        // Fallback - assume running if we have a handle
        true
    }
}

/// Cleanup Arti process (called during app shutdown)
pub async fn cleanup_arti() {
    if let Err(e) = stop_arti().await {
        warn!("⚠️ Failed to cleanup Arti during shutdown: {}", e);
    } else {
        info!("✅ Arti cleanup completed");
    }
}