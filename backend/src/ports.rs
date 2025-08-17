use crate::paths::PathConfig;
use crate::state::AppState;
use std::env;
use std::fs;
use std::process::Command;

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
            eprintln!(
                "⚠️  Warning: Could not parse torrc file {}: {}",
                torrc_path, e
            );
            eprintln!("   Using default Tor ports instead");

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

/// Check if a port is in use on macOS
pub fn is_port_in_use(port: u16) -> Result<bool, String> {
    let output = Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-t"])
        .output()
        .map_err(|e| format!("Failed to run lsof: {}", e))?;

    Ok(!output.stdout.is_empty())
}

/// Get the PID of the process using a specific port
pub fn get_pid_using_port(port: u16) -> Result<Option<u32>, String> {
    let output = Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-t"])
        .output()
        .map_err(|e| format!("Failed to run lsof: {}", e))?;

    if output.stdout.is_empty() {
        return Ok(None);
    }

    let pid_str = String::from_utf8_lossy(&output.stdout);
    let pid_str = pid_str.trim();

    if let Ok(pid) = pid_str.parse::<u32>() {
        Ok(Some(pid))
    } else {
        Ok(None)
    }
}

/// Get process information for a given PID
pub fn get_process_info(pid: u32) -> Result<String, String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map_err(|e| format!("Failed to run ps: {}", e))?;

    let process_name = String::from_utf8_lossy(&output.stdout);
    Ok(process_name.trim().to_string())
}

/// Kill a process by PID
pub fn kill_process(pid: u32) -> Result<(), String> {
    let output = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to kill process {}: {}", pid, e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to kill process {}: {}", pid, error));
    }

    // Wait a moment and try SIGKILL if process is still running
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_port_in_use_by_pid(pid)? {
        let output = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output()
            .map_err(|e| format!("Failed to force kill process {}: {}", pid, e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to force kill process {}: {}", pid, error));
        }
    }

    Ok(())
}

/// Check if a specific PID is still running
fn is_port_in_use_by_pid(pid: u32) -> Result<bool, String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to check if PID {} exists: {}", pid, e))?;

    Ok(output.status.success())
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
    println!(
        "🔍 Checking for processes using application ports (torrc: {})...",
        torrc_filename
    );

    let ports = get_ports_to_check_with_torrc(torrc_filename)?;
    let mut killed_processes = 0;

    for port_info in ports {
        match is_port_in_use(port_info.port) {
            Ok(true) => match get_pid_using_port(port_info.port) {
                Ok(Some(pid)) => match get_process_info(pid) {
                    Ok(process_name) => {
                        println!(
                            "🔥 Port {} ({}) is in use by PID {} ({})",
                            port_info.port, port_info.description, pid, process_name
                        );

                        print!("   Killing process... ");
                        match kill_process(pid) {
                            Ok(()) => {
                                println!("✅ Killed successfully");
                                killed_processes += 1;
                            }
                            Err(e) => {
                                println!("❌ Failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Port {} is in use by PID {} but couldn't get process info: {}",
                            port_info.port, pid, e
                        );
                    }
                },
                Ok(None) => {
                    println!(
                        "⚠️  Port {} appears to be in use but no PID found",
                        port_info.port
                    );
                }
                Err(e) => {
                    println!("❌ Error checking PID for port {}: {}", port_info.port, e);
                }
            },
            Ok(false) => {
                println!(
                    "✅ Port {} ({}) is available",
                    port_info.port, port_info.description
                );
            }
            Err(e) => {
                println!("❌ Error checking port {}: {}", port_info.port, e);
            }
        }
    }

    if killed_processes > 0 {
        println!(
            "🧹 Cleanup completed: killed {} processes",
            killed_processes
        );
        // Give processes time to fully terminate
        std::thread::sleep(std::time::Duration::from_millis(1000));
    } else {
        println!("✨ All ports are clean - no cleanup needed");
    }

    Ok(())
}

/// Clean up only Tor-related ports, leaving phoenixd running
pub async fn cleanup_tor_ports_only(torrc_filename: &str) -> Result<(), String> {
    println!(
        "🔍 Checking for Tor processes only (torrc: {}, preserving phoenixd)...",
        torrc_filename
    );

    // Get only Tor ports, excluding phoenixd
    let tor_ports = get_tor_ports_only(torrc_filename)?;
    let mut killed_processes = 0;

    for port_info in tor_ports {
        match is_port_in_use(port_info.port) {
            Ok(true) => match get_pid_using_port(port_info.port) {
                Ok(Some(pid)) => match get_process_info(pid) {
                    Ok(process_name) => {
                        println!(
                            "🔥 Port {} ({}) is in use by PID {} ({})",
                            port_info.port, port_info.description, pid, process_name
                        );

                        print!("   Killing process... ");
                        match kill_process(pid) {
                            Ok(()) => {
                                println!("✅ Killed successfully");
                                killed_processes += 1;
                            }
                            Err(e) => {
                                println!("❌ Failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Port {} is in use by PID {} but couldn't get process info: {}",
                            port_info.port, pid, e
                        );
                    }
                },
                Ok(None) => {
                    println!(
                        "⚠️  Port {} appears to be in use but no PID found",
                        port_info.port
                    );
                }
                Err(e) => {
                    println!("❌ Error checking PID for port {}: {}", port_info.port, e);
                }
            },
            Ok(false) => {
                println!(
                    "✅ Port {} ({}) is available",
                    port_info.port, port_info.description
                );
            }
            Err(e) => {
                println!("❌ Error checking port {}: {}", port_info.port, e);
            }
        }
    }

    if killed_processes > 0 {
        println!("🧹 Killed {} Tor processes", killed_processes);

        // Wait a moment for processes to clean up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    } else {
        println!("ℹ️  No Tor processes needed to be killed");
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
            eprintln!(
                "⚠️  Warning: Could not parse torrc file {}: {}",
                torrc_path, e
            );
            eprintln!("   Using default Tor ports instead");

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
    println!("🔍 Checking if backend port {} is in use...", port);
    
    match is_port_in_use(port) {
        Ok(false) => {
            println!("✅ Backend port {} is available", port);
            Ok(())
        }
        Ok(true) => {
            println!("⚠️  Backend port {} is in use, attempting to free it...", port);
            
            match get_pid_using_port(port) {
                Ok(Some(pid)) => {
                    println!("🔍 Found process {} using backend port {}", pid, port);
                    match kill_process(pid) {
                        Ok(_) => {
                            println!("✅ Successfully killed process {} on backend port {}", pid, port);
                            Ok(())
                        }
                        Err(e) => {
                            let error = format!("Failed to kill process {} on backend port {}: {}", pid, port, e);
                            println!("❌ {}", error);
                            Err(error)
                        }
                    }
                }
                Ok(None) => {
                    let error = format!("Backend port {} is in use but no PID found", port);
                    println!("⚠️  {}", error);
                    Err(error)
                }
                Err(e) => {
                    let error = format!("Failed to get PID for backend port {}: {}", port, e);
                    println!("❌ {}", error);
                    Err(error)
                }
            }
        }
        Err(e) => {
            let error = format!("Failed to check backend port {}: {}", port, e);
            println!("❌ {}", error);
            Err(error)
        }
    }
}
