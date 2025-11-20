use crate::paths::PathConfig;
use crate::state::AppState;
use std::env;
use std::fs;
use log::info;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags};
use sysinfo::{System, Pid};


/// Default ports used by the application
const DEFAULT_PHOENIXD_PORT: u16 = 9740;
const DEFAULT_TOR_SOCKS_PORT: u16 =  18058;
const DEFAULT_TOR_CONTROL_PORT: u16 = 9992;

const DEFAULT_TOR_RELAY_SOCKS_PORT: u16 = 18057;
const DEFAULT_TOR_RELAY_CONTROL_PORT: u16 = 7781;

/// Represents a port that needs to be checked/killed
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    #[allow(dead_code)]
    pub service_name: String,
    pub description: String,
}

/// Parse the torrc file to extract SocksPort and ControlPort
pub fn parse_torrc_ports(torrc_path: &str) -> Result<Vec<PortInfo>, String> {
    let mut ports = Vec::new();

    let content = fs::read_to_string(torrc_path)
        .map_err(|e| format!("Failed to read torrc file {}: {}", torrc_path, e))?;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.starts_with(';') || line.is_empty() {
            continue;
        }

        // Parse SocksPort
        if line.starts_with("SocksPort ") {
            if let Some(port_str) = line.split_whitespace().nth(1) {
                if let Ok(port) = port_str.parse::<u16>() {
                    ports.push(PortInfo {
                        port,
                        service_name: "tor".to_string(),
                        description: "Tor SOCKS Port".to_string(),
                    });
                }
            }
        }

        // Parse ControlPort
        if line.starts_with("ControlPort ") {
            if let Some(port_str) = line.split_whitespace().nth(1) {
                if let Ok(port) = port_str.parse::<u16>() {
                    ports.push(PortInfo {
                        port,
                        service_name: "tor".to_string(),
                        description: "Tor Control Port".to_string(),
                    });
                }
            }
        }
    }

    Ok(ports)
}

/// Get the phoenixd port from environment or use default
pub fn get_phoenixd_port() -> u16 {
    env::var("PHOENIX_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PHOENIXD_PORT)
}

/// Get all ports that need to be checked using AppState torrc configuration
pub fn get_ports_to_check(app_state: &AppState) -> Result<Vec<PortInfo>, String> {
    get_ports_to_check_with_torrc(&app_state.torrc_file_name)
}

/// Get all ports that need to be checked with a specific torrc filename
pub fn get_ports_to_check_with_torrc(torrc_filename: &str) -> Result<Vec<PortInfo>, String> {
    let mut ports = Vec::new();

    // Add phoenixd port
    let phoenixd_port = get_phoenixd_port();
    let use_phoenixd_embedded = env::var("APP_ELTOR_USE_PHOENIXD_EMBEDDED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    if use_phoenixd_embedded {
        ports.push(PortInfo {
            port: phoenixd_port,
            service_name: "phoenixd".to_string(),
            description: "Phoenix Lightning Node".to_string(),
        });
    }

    // Parse torrc file for Tor ports
    let path_config = PathConfig::new().map_err(|e| e)?;
    let torrc_path = path_config.get_torrc_path_for_ports(torrc_filename);
    match parse_torrc_ports(&torrc_path) {
        Ok(mut tor_ports) => ports.append(&mut tor_ports),
        Err(e) => {
            info!(
                "⚠️  Warning: Could not parse torrc file {}: {}",
                torrc_path, e
            );
            info!("   Using default Tor ports instead");

            // Use default ports if torrc parsing fails
            ports.push(PortInfo {
                port: DEFAULT_TOR_SOCKS_PORT,
                service_name: "tor".to_string(),
                description: "Tor SOCKS Port (default)".to_string(),
            });
            ports.push(PortInfo {
                port: DEFAULT_TOR_CONTROL_PORT,
                service_name: "tor".to_string(),
                description: "Tor Control Port (default)".to_string(),
            });
             ports.push(PortInfo {
                port: DEFAULT_TOR_RELAY_SOCKS_PORT,
                service_name: "tor".to_string(),
                description: "Tor Relay SOCKS Port (default)".to_string(),
            });
            ports.push(PortInfo {
                port: DEFAULT_TOR_RELAY_CONTROL_PORT,
                service_name: "tor".to_string(),
                description: "Tor Relay Control Port (default)".to_string(),
            });
        }
    }

    Ok(ports)
}

/// Check if a port is in use (cross-platform)
pub fn is_port_in_use(port: u16) -> Result<bool, String> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    
    let sockets = get_sockets_info(af_flags, proto_flags)
        .map_err(|e| format!("Failed to get socket info: {}", e))?;
    
    Ok(sockets.iter().any(|s| s.local_port() == port))
}

/// Get the PID of the process using a specific port (cross-platform)
pub async fn get_pid_using_port(port: u16) -> Result<Option<u32>, String> {
    tokio::task::spawn_blocking(move || {
        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        
        let sockets = get_sockets_info(af_flags, proto_flags)
            .map_err(|e| format!("Failed to get socket info: {}", e))?;
        
        for socket in sockets {
            if socket.local_port() == port {
                if let Some(&pid) = socket.associated_pids.first() {
                    return Ok(Some(pid));
                }
            }
        }
        
        Ok(None)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Get process information for a given PID (cross-platform)
pub fn get_process_info(pid: u32) -> Result<String, String> {
    let system = System::new_all();
    
    if let Some(process) = system.process(Pid::from_u32(pid)) {
        Ok(process.name().to_string_lossy().to_string())
    } else {
        Err(format!("Process {} not found", pid))
    }
}

/// Kill a process by PID (cross-platform)
pub fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::io::ErrorKind;
        
        // On Unix, we need to kill both the process AND its process group
        // This handles processes created with setsid() that become session leaders
        
        // Step 1: Try to kill the process group first (negative PID)
        // This kills all children if the process is a session leader
        let pgid = -(pid as i32);
        
        log::info!("🔪 Attempting to kill process {} and its process group", pid);
        
        // Try graceful kill on process group first (SIGTERM)
        let pg_killed = unsafe {
            if libc::kill(pgid, libc::SIGTERM) != -1 {
                log::info!("   Sent SIGTERM to process group {}", pid);
                true
            } else {
                let err = std::io::Error::last_os_error();
                if err.kind() == ErrorKind::PermissionDenied || err.raw_os_error() == Some(libc::ESRCH) {
                    log::info!("   No process group found for {}, will try individual process", pid);
                    false
                } else {
                    log::warn!("   Failed to kill process group {}: {}", pid, err);
                    false
                }
            }
        };
        
        // Step 2: Also kill the main process individually (some processes might not be in their own group)
        unsafe {
            if libc::kill(pid as i32, libc::SIGTERM) == -1 {
                let err = std::io::Error::last_os_error();
                if err.kind() != ErrorKind::NotFound {
                    log::warn!("   Failed to send SIGTERM to process {}: {}", pid, err);
                }
            } else {
                log::info!("   Sent SIGTERM to process {}", pid);
            }
        }
        
        // Wait a moment for graceful shutdown
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Step 3: Check if process is still running (not a zombie), force kill if necessary
        #[cfg(target_os = "linux")]
        let still_running = {
            // On Linux, check /proc/{pid}/stat to see if it's not a zombie
            let proc_stat = format!("/proc/{}/stat", pid);
            if let Ok(stat) = std::fs::read_to_string(&proc_stat) {
                if let Some(state_start) = stat.rfind(')') {
                    if let Some(state_char) = stat.chars().nth(state_start + 2) {
                        // Not a zombie or dead process
                        state_char != 'Z' && state_char != 'X'
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false // Process doesn't exist
            }
        };
        
        #[cfg(not(target_os = "linux"))]
        let still_running = unsafe { libc::kill(pid as i32, 0) == 0 };
        
        if still_running {
            // Process still exists and is not a zombie, force kill
            log::warn!("⚠️ Process {} did not exit gracefully, sending SIGKILL", pid);
            
            unsafe {
                // Kill process group with SIGKILL
                if pg_killed {
                    if libc::kill(pgid, libc::SIGKILL) == -1 {
                        let err = std::io::Error::last_os_error();
                        if err.kind() != ErrorKind::NotFound {
                            log::warn!("   Failed to SIGKILL process group {}: {}", pid, err);
                        }
                    } else {
                        log::info!("   Sent SIGKILL to process group {}", pid);
                    }
                }
                
                // Kill individual process with SIGKILL
                if libc::kill(pid as i32, libc::SIGKILL) == -1 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() != ErrorKind::NotFound {
                        return Err(format!("Failed to SIGKILL process {}: {}", pid, err));
                    }
                } else {
                    log::info!("   Sent SIGKILL to process {}", pid);
                }
            }
            
            // Wait a bit more for the kill to take effect
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            log::info!("✅ Process {} terminated gracefully", pid);
        }
        
        // Step 4: Try to reap zombie processes by calling waitpid with WNOHANG
        // This helps clean up any zombie processes that might be left behind
        #[cfg(target_os = "linux")]
        unsafe {
            let mut status: libc::c_int = 0;
            // Use WNOHANG to avoid blocking if the process is not a child
            // Use __WALL to wait for all children regardless of type
            let result = libc::waitpid(pid as i32, &mut status, libc::WNOHANG | libc::__WALL);
            if result > 0 {
                log::info!("🧹 Reaped zombie process {}", pid);
            } else if result == 0 {
                // Process still exists but hasn't terminated yet - this is fine
                log::debug!("Process {} still running or already reaped", pid);
            }
            // If result is -1, it means we can't wait for this process (not our child)
            // which is fine - the init process will eventually reap it
        }
        
        Ok(())
    }
    
    #[cfg(not(unix))]
    {
        let system = System::new_all();
        
        if let Some(process) = system.process(Pid::from_u32(pid)) {
            // Try graceful kill first (TerminateProcess on Windows)
            if !process.kill() {
                return Err(format!("Failed to kill process {}", pid));
            }
            
            // Wait a moment and verify
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Refresh and check if still running
            let system = System::new_all();
            if system.process(Pid::from_u32(pid)).is_some() {
                // Force kill if still running
                if let Some(process) = system.process(Pid::from_u32(pid)) {
                    process.kill();
                }
            }
            
            Ok(())
        } else {
            Ok(()) // Process already dead
        }
    }
}

/// Check if a specific PID is still running (cross-platform)
#[allow(dead_code)]
fn is_port_in_use_by_pid(pid: u32) -> Result<bool, String> {
    let system = System::new_all();
    Ok(system.process(Pid::from_u32(pid)).is_some())
}

/// Clean up ports used by the application using AppState torrc configuration
pub async fn cleanup_ports(app_state: &AppState) -> Result<(), String> {
    cleanup_ports_with_torrc(&app_state.torrc_file_name).await
}

/// Clean up ports during startup using default torrc file
pub async fn cleanup_ports_startup() -> Result<(), String> {
    cleanup_ports_with_torrc("torrc").await
}

/// Clean up ports used by the application for a specific torrc file
pub async fn cleanup_ports_with_torrc(torrc_filename: &str) -> Result<(), String> {
    info!(
        "🔍 Checking for processes using application ports (torrc: {})...",
        torrc_filename
    );

    let ports = get_ports_to_check_with_torrc(torrc_filename)?;
    let mut killed_processes = 0;

    for port_info in ports {
        match is_port_in_use(port_info.port) {
            Ok(true) => match get_pid_using_port(port_info.port).await {
                Ok(Some(pid)) => match get_process_info(pid) {
                    Ok(process_name) => {
                        info!(
                            "🔥 Port {} ({}) is in use by PID {} ({})",
                            port_info.port, port_info.description, pid, process_name
                        );

                        // print!("   Killing process... ");
                        match kill_process(pid) {
                            Ok(()) => {
                                info!("✅ Killed successfully");
                                killed_processes += 1;
                            }
                            Err(e) => {
                                info!("❌ Failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        info!(
                            "⚠️  Port {} is in use by PID {} but couldn't get process info: {}",
                            port_info.port, pid, e
                        );
                    }
                },
                Ok(None) => {
                    info!(
                        "⚠️  Port {} appears to be in use but no PID found",
                        port_info.port
                    );
                }
                Err(e) => {
                    info!("❌ Error checking PID for port {}: {}", port_info.port, e);
                }
            },
            Ok(false) => {
                info!(
                    "✅ Port {} ({}) is available",
                    port_info.port, port_info.description
                );
            }
            Err(e) => {
                info!("❌ Error checking port {}: {}", port_info.port, e);
            }
        }
    }

    if killed_processes > 0 {
        info!(
            "🧹 Cleanup completed: killed {} processes",
            killed_processes
        );
        // Give processes time to fully terminate
        std::thread::sleep(std::time::Duration::from_millis(1000));
    } else {
        info!("✨ All ports are clean - no cleanup needed");
    }

    Ok(())
}

/// Clean up only Tor-related ports, leaving phoenixd running
pub async fn cleanup_tor_ports_only(torrc_filename: &str) -> Result<(), String> {
    info!(
        "🔍 Checking for Tor processes only (torrc: {}, preserving phoenixd)...",
        torrc_filename
    );

    // Get only Tor ports, excluding phoenixd
    let tor_ports = get_tor_ports_only(torrc_filename)?;
    let mut killed_processes = 0;

    for port_info in tor_ports {
        match is_port_in_use(port_info.port) {
            Ok(true) => match get_pid_using_port(port_info.port).await {
                Ok(Some(pid)) => match get_process_info(pid) {
                    Ok(process_name) => {
                        info!(
                            "🔥 Port {} ({}) is in use by PID {} ({})",
                            port_info.port, port_info.description, pid, process_name
                        );

                        // print!("   Killing process... ");
                        match kill_process(pid) {
                            Ok(()) => {
                                info!("✅ Killed successfully");
                                killed_processes += 1;
                            }
                            Err(e) => {
                                info!("❌ Failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        info!(
                            "⚠️  Port {} is in use by PID {} but couldn't get process info: {}",
                            port_info.port, pid, e
                        );
                    }
                },
                Ok(None) => {
                    info!(
                        "⚠️  Port {} appears to be in use but no PID found",
                        port_info.port
                    );
                }
                Err(e) => {
                    info!("❌ Error checking PID for port {}: {}", port_info.port, e);
                }
            },
            Ok(false) => {
                info!(
                    "✅ Port {} ({}) is available",
                    port_info.port, port_info.description
                );
            }
            Err(e) => {
                info!("❌ Error checking port {}: {}", port_info.port, e);
            }
        }
    }

    if killed_processes > 0 {
        info!("🧹 Killed {} Tor processes", killed_processes);

        // Wait a moment for processes to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    } else {
        info!("ℹ️  No Tor processes needed to be killed");
    }

    Ok(())
}

/// Get only Tor ports from torrc, excluding phoenixd
pub fn get_tor_ports_only(torrc_filename: &str) -> Result<Vec<PortInfo>, String> {
    let mut ports = Vec::new();

    // Parse torrc file for Tor ports only (no phoenixd)
    let path_config = PathConfig::new().map_err(|e| e)?;
    let torrc_path = path_config.get_torrc_path_for_ports(torrc_filename);
    match parse_torrc_ports(&torrc_path) {
        Ok(mut tor_ports) => ports.append(&mut tor_ports),
        Err(e) => {
            info!(
                "⚠️  Warning: Could not parse torrc file {}: {}",
                torrc_path, e
            );
            info!("   Using default Tor ports instead");

            // Use default ports if torrc parsing fails
            ports.push(PortInfo {
                port: DEFAULT_TOR_SOCKS_PORT,
                service_name: "tor".to_string(),
                description: "Tor SOCKS Port (default)".to_string(),
            });
            ports.push(PortInfo {
                port: DEFAULT_TOR_CONTROL_PORT,
                service_name: "tor".to_string(),
                description: "Tor Control Port (default)".to_string(),
            });
            ports.push(PortInfo {
                port: DEFAULT_TOR_RELAY_SOCKS_PORT,
                service_name: "tor".to_string(),
                description: "Tor Relay SOCKS Port (default)".to_string(),
            });
            ports.push(PortInfo {
                port: DEFAULT_TOR_RELAY_CONTROL_PORT,
                service_name: "tor".to_string(),
                description: "Tor Relay Control Port (default)".to_string(),
            });
        }
    }

    Ok(ports)
}

/// Kill any process using the backend server port
pub async fn cleanup_backend_port(port: u16) -> Result<(), String> {
    info!("🔍 Checking if backend port {} is in use...", port);
    
    match is_port_in_use(port) {
        Ok(false) => {
            info!("✅ Backend port {} is available", port);
            Ok(())
        }
        Ok(true) => {
            info!("⚠️  Backend port {} is in use, attempting to free it...", port);
            
            match get_pid_using_port(port).await {
                Ok(Some(pid)) => {
                    info!("🔍 Found process {} using backend port {}", pid, port);
                    match kill_process(pid) {
                        Ok(_) => {
                            info!("✅ Successfully killed process {} on backend port {}", pid, port);
                            Ok(())
                        }
                        Err(e) => {
                            let error = format!("Failed to kill process {} on backend port {}: {}", pid, port, e);
                            info!("❌ {}", error);
                            Err(error)
                        }
                    }
                }
                Ok(None) => {
                    let error = format!("Backend port {} is in use but no PID found", port);
                    info!("⚠️  {}", error);
                    Err(error)
                }
                Err(e) => {
                    let error = format!("Failed to get PID for backend port {}: {}", port, e);
                    info!("❌ {}", error);
                    Err(error)
                }
            }
        }
        Err(e) => {
            let error = format!("Failed to check backend port {}: {}", port, e);
            info!("❌ {}", error);
            Err(error)
        }
    }
}
