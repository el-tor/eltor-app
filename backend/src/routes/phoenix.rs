use std::fs;
use std::io::Cursor;
use std::path::Path;
use tokio::task;
use zip::ZipArchive;
use log::info;


use axum::{
    extract::State,
    http::StatusCode,
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use serde::Serialize;
use tokio::process::Command as TokioCommand;
use std::process::Stdio;

use crate::{paths::PathConfig, state::{AppState, LogEntry}};
use chrono::Utc;

const PHOENIX_VERSION: &str = "0.6.1";
const PHOENIX_BASE_URL: &str = "https://github.com/ACINQ/phoenixd/releases/download";
const PHOENIX_DEFAULT_URL: &str = "http://127.0.0.1:9740";

/// Try to get Phoenix configuration from existing running instance
async fn get_existing_phoenix_config() -> Result<(String, String), String> {
    let home_dir = dirs::home_dir().ok_or("Could not get home directory")?;
    let phoenix_conf_path = home_dir.join(".phoenix").join("phoenix.conf");
    
    if !phoenix_conf_path.exists() {
        return Err("Phoenix config file not found".to_string());
    }
    
    let password = get_phoenixd_password()?;
    let default_url = PHOENIX_DEFAULT_URL.to_string();
    
    Ok((default_url, password))
}

/// Phoenix download module for eltor-backend
/// 
/// This module provides functionality to automatically download and extract
/// the appropriate Phoenix release for the current platform and architecture.
/// 
/// # Example Usage
/// 
/// ```rust,no_run
/// use eltor_backend::routes::phoenix;
/// use eltor_backend::paths::PathConfig;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Using custom PathConfig
///     let path_config = PathConfig::new()?;
///     match phoenix::download_phoenix(&path_config).await {
///         Ok(msg) => info!("{}", msg),
///         Err(e) => info!("Download failed: {}", e),
///     }
/// 
///     // Using default PathConfig
///     match phoenix::download_phoenix_default().await {
///         Ok(msg) => info!("{}", msg),
///         Err(e) => info!("Download failed: {}", e),
///     }
/// 
///     Ok(())
/// }
/// ```
/// 
/// # Supported Platforms
/// 
/// - **macOS**: x64 and ARM64 architectures
/// - **Linux**: x64 and ARM64 architectures  
/// - **Windows**: Uses JVM version (cross-platform)
/// 
/// The function automatically detects your platform and downloads the correct release.

/// Download the appropriate Phoenix release for the current platform
pub async fn download_phoenix(path_config: &PathConfig) -> Result<String, String> {
    let (platform, arch) = detect_platform_and_arch()?;
    let download_url = get_download_url(&platform, &arch)?;
    
    info!("🔥 Downloading Phoenix {} for {}-{}...", PHOENIX_VERSION, platform, arch);
    info!("📥 Download URL: {}", download_url);
    
    // Download the file
    let zip_data = download_file(&download_url).await?;
    
    // Extract to bin directory
    extract_phoenix_archive(&zip_data, &path_config.bin_dir, &platform).await?;
    
    let success_msg = format!(
        "✅ Phoenix {} successfully downloaded and extracted to {:?}",
        PHOENIX_VERSION,
        path_config.bin_dir
    );
    info!("{}", success_msg);
    Ok(success_msg)
}

/// Convenience function to download Phoenix using default PathConfig
pub async fn download_phoenix_default() -> Result<String, String> {
    let path_config = PathConfig::new()
        .map_err(|e| format!("Failed to create PathConfig: {}", e))?;
    download_phoenix(&path_config).await
}

/// Detect the current platform and architecture
fn detect_platform_and_arch() -> Result<(String, String), String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    let (platform, mapped_arch) = match (os, arch) {
        ("macos", "x86_64") => ("macos", "x64"),
        ("macos", "aarch64") => ("macos", "arm64"),
        ("linux", "x86_64") => ("linux", "x64"),
        ("linux", "aarch64") => ("linux", "arm64"),
        ("windows", _) => ("windows", "jvm"), // Windows uses JVM version
        _ => return Err(format!("Unsupported platform: {}-{}", os, arch)),
    };
    
    Ok((platform.to_string(), mapped_arch.to_string()))
}

/// Generate the download URL for the given platform and architecture
fn get_download_url(platform: &str, arch: &str) -> Result<String, String> {
    let filename = if platform == "windows" {
        format!("phoenixd-{}-{}.zip", PHOENIX_VERSION, arch)
    } else {
        format!("phoenixd-{}-{}-{}.zip", PHOENIX_VERSION, platform, arch)
    };
    
    Ok(format!("{}/v{}/{}", PHOENIX_BASE_URL, PHOENIX_VERSION, filename))
}

/// Download file from URL using reqwest
async fn download_file(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }
    
    let total_size = response.content_length();
    if let Some(size) = total_size {
        info!("📦 Download size: {} bytes ({:.2} MB)", size, size as f64 / 1_048_576.0);
    }
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    
    Ok(bytes.to_vec())
}

/// Extract Phoenix archive to the bin directory
async fn extract_phoenix_archive(
    zip_data: &[u8], 
    bin_dir: &Path, 
    platform: &str
) -> Result<(), String> {
    // Ensure bin directory exists
    if let Err(e) = fs::create_dir_all(bin_dir) {
        return Err(format!("Failed to create bin directory: {}", e));
    }
    
    // Clone data for the blocking task
    let zip_data = zip_data.to_vec();
    let bin_dir = bin_dir.to_path_buf();
    let platform = platform.to_string();
    
    // Use tokio::task::spawn_blocking for CPU-intensive zip extraction
    task::spawn_blocking(move || {
        extract_zip_blocking(&zip_data, &bin_dir, &platform)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Blocking zip extraction function
fn extract_zip_blocking(zip_data: &[u8], bin_dir: &Path, platform: &str) -> Result<(), String> {
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;
    
    info!("📂 Extracting {} files from Phoenix archive...", archive.len());
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read file from archive: {}", e))?;
        
        let file_name = file.name().to_string();
        
        if file_name.ends_with('/') {
            // Skip directories entirely - we'll create them as needed when extracting files
            continue;
        }
        
        let original_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => {
                info!("⚠️  Skipping file with invalid name: {}", file_name);
                continue;
            }
        };
        
        // For all files, extract just the filename to bin root (flatten the directory structure)
        let filename = original_path.file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        
        if filename.is_empty() {
            info!("⚠️  Skipping file with empty filename: {}", file_name);
            continue;
        }
        
        // Extract all files directly to bin root
        let outpath = bin_dir.join(&filename);
        
        // Extract the file
        let mut outfile = fs::File::create(&outpath)
            .map_err(|e| format!("Failed to create file {:?}: {}", outpath, e))?;
        
        std::io::copy(&mut file, &mut outfile)
            .map_err(|e| format!("Failed to extract file {:?}: {}", outpath, e))?;
        
        // Set executable permissions for phoenixd and phoenix-cli (Unix-like systems only)
        let is_phoenixd_executable = filename == "phoenixd" || 
                                     filename == "phoenix-cli" ||
                                     (platform == "windows" && (filename == "phoenixd.exe" || filename == "phoenix-cli.exe"));
        
        if platform != "windows" && is_phoenixd_executable {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = outfile.metadata()
                    .map_err(|e| format!("Failed to get file metadata: {}", e))?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o755); // rwxr-xr-x
                fs::set_permissions(&outpath, perms)
                    .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
                info!("🔧 Set executable permissions for: {:?}", outpath);
            }
        }
        
        info!("📄 Extracted to bin root: {:?}", outpath);
    }
    
    Ok(())
}

/// Parse the phoenix.conf file to extract the HTTP password
/// Based on the logic from start.sh: awk -F'=' '/^http-password=/ {print $2}' ~/.phoenix/phoenix.conf
fn get_phoenixd_password() -> Result<String, String> {
    let home_dir = dirs::home_dir()
        .ok_or("Failed to get home directory")?;
    
    let phoenix_conf_path = home_dir.join(".phoenix").join("phoenix.conf");
    
    if !phoenix_conf_path.exists() {
        return Err(format!("Phoenix config file not found at: {:?}", phoenix_conf_path));
    }
    
    let conf_content = std::fs::read_to_string(&phoenix_conf_path)
        .map_err(|e| format!("Failed to read phoenix.conf: {}", e))?;
    
    // Parse the config file line by line looking for http-password=
    for line in conf_content.lines() {
        let line = line.trim();
        if line.starts_with("http-password=") {
            // Split on '=' and take everything after the first '='
            if let Some(password) = line.strip_prefix("http-password=") {
                return Ok(password.to_string());
            }
        }
    }
    
    Err("http-password not found in phoenix.conf".to_string())
}

/// Wait for phoenix.conf to be created and return the password
/// This function will wait up to 10 seconds for the config file to appear
async fn wait_for_phoenix_conf_and_get_password() -> Result<String, String> {
    for attempt in 1..=10 {
        info!("🔍 Attempt {} to read Phoenix password from config...", attempt);
        
        match get_phoenixd_password() {
            Ok(password) => {
                info!("✅ Successfully read Phoenix password from ~/.phoenix/phoenix.conf");
                return Ok(password);
            }
            Err(e) => {
                if attempt < 10 {
                    info!("⏳ Phoenix config not ready yet: {}. Waiting 1 second...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                } else {
                    return Err(format!("Failed to get Phoenix password after {} attempts: {}", attempt, e));
                }
            }
        }
    }
    
    Err("Failed to get Phoenix password: maximum attempts exceeded".to_string())
}

/// Public function for Tauri to start Phoenix and get configuration
/// Returns the same structure as the API endpoint but without the HTTP wrapper
pub async fn start_phoenix_with_config(path_config: &PathConfig) -> Result<PhoenixStartResponse, String> {
    // Create a minimal AppState for the Phoenix process management
    // This is a simplified version for Tauri usage
    use crate::state::AppState;
    let app_state = AppState::new(true, path_config.clone()); // Enable embedded phoenixd
    
    // Check if phoenixd binary exists, download if needed
    let phoenixd_binary = path_config.get_executable_path("phoenixd");
    let mut downloaded = false;

    if !phoenixd_binary.exists() {
        info!("📥 Phoenix binary not found, downloading...");
        
        match download_phoenix(path_config).await {
            Ok(_) => {
                downloaded = true;
            }
            Err(e) => {
                return Err(format!("Failed to download Phoenix: {}", e));
            }
        }
    }

    // Start the phoenixd process
    match start_phoenixd_process(&app_state, path_config).await {
        Ok((pid, url, password)) => {
            Ok(PhoenixStartResponse {
                success: true,
                message: format!("Phoenix daemon started successfully with PID: {}", pid),
                downloaded,
                pid: Some(pid),
                url: Some(url),
                password: if password.is_empty() { None } else { Some(password) },
                is_running: Some(true),
            })
        }
        Err(e) => {
            Err(format!("Failed to start Phoenix daemon: {}", e))
        }
    }
}

/// API endpoint response for Phoenix start operation
#[derive(Serialize, Debug)]
pub struct PhoenixStartResponse {
    pub success: bool,
    pub message: String,
    pub downloaded: bool,
    pub pid: Option<u32>,
    pub url: Option<String>,
    pub password: Option<String>,
    pub is_running: Option<bool>,
}

/// API endpoint response for Phoenix stop operation
#[derive(Serialize)]
pub struct PhoenixStopResponse {
    pub success: bool,
    pub message: String,
    pub pid: Option<u32>,
}

/// Create router for Phoenix API endpoints
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/phoenix/start", post(start_phoenix_api))
        .route("/api/phoenix/stop", post(stop_phoenix_api))
        .route("/api/phoenix/detect-config", post(detect_phoenix_config_api))
}

/// API endpoint to start Phoenix daemon
/// This will check if phoenixd binary exists, download it if needed, and start the process
async fn start_phoenix_api(State(state): State<AppState>) -> Result<ResponseJson<PhoenixStartResponse>, StatusCode> {
    // Check if phoenixd is already running
    {
        let process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        if let Some(ref child) = *process_guard {
            // For simplicity, we'll assume if there's a process it's still running
            // In production you might want to check child.try_wait()
            return Ok(ResponseJson(PhoenixStartResponse {
                success: true,
                message: "Phoenix daemon is already running".to_string(),
                downloaded: false,
                pid: child.id(),
                url: Some(PHOENIX_DEFAULT_URL.to_string()),
                password: None, // Don't expose existing password in API response
                is_running: Some(true),
            }));
        }
    }

    // Get path configuration
    let path_config = match PathConfig::new() {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("Failed to get path configuration: {}", e);
            state.add_log(LogEntry {
                timestamp: Utc::now(),
                level: "ERROR".to_string(),
                message: error_msg.clone(),
                source: "phoenix-api".to_string(),
                mode: None,
            });
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Check if phoenixd binary exists
    let phoenixd_binary = path_config.get_executable_path("phoenixd");
    let mut downloaded = false;

    if !phoenixd_binary.exists() {
        info!("📥 Phoenix binary not found, downloading...");
        state.add_log(LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "Phoenix binary not found, downloading...".to_string(),
            source: "phoenix-api".to_string(),
            mode: None,
        });

        // Download and extract Phoenix
        match download_phoenix(&path_config).await {
            Ok(msg) => {
                info!("{}", msg);
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: msg,
                    source: "phoenix-api".to_string(),
                    mode: None,
                });
                downloaded = true;
            }
            Err(e) => {
                let error_msg = format!("Failed to download Phoenix: {}", e);
                info!("❌ {}", error_msg);
                state.add_log(LogEntry {
                    timestamp: Utc::now(),
                    level: "ERROR".to_string(),
                    message: error_msg,
                    source: "phoenix-api".to_string(),
                    mode: None,
                });
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Now start the phoenixd process
    match start_phoenixd_process(&state, &path_config).await {
        Ok((pid, url, password)) => {
            let success_msg = format!("Phoenix daemon started successfully with PID: {}", pid);
            info!("✅ {}", success_msg);
            state.add_log(LogEntry {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                message: success_msg.clone(),
                source: "phoenix-api".to_string(),
                mode: None,
            });

            Ok(ResponseJson(PhoenixStartResponse {
                success: true,
                message: success_msg,
                downloaded,
                pid: Some(pid),
                url: Some(url),
                password: if password.is_empty() { None } else { Some(password) },
                is_running: Some(true),
            }))
        }
        Err(e) => {
            let error_msg = format!("Failed to start Phoenix daemon: {}", e);
            info!("❌ {}", error_msg);
            state.add_log(LogEntry {
                timestamp: Utc::now(),
                level: "ERROR".to_string(),
                message: error_msg,
                source: "phoenix-api".to_string(),
                mode: None,
            });
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// API endpoint to stop Phoenix daemon
/// This will gracefully terminate the phoenixd process if it's running
/// API endpoint to stop Phoenix daemon
/// This will gracefully terminate the phoenixd process if it's running
async fn stop_phoenix_api(State(state): State<AppState>) -> Result<ResponseJson<PhoenixStopResponse>, StatusCode> {
    // Check if phoenixd is currently running and take ownership of the process
    let child_process = {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        process_guard.take()
    };
    
    match child_process {
        Some(mut child) => {
            let pid = child.id();
            
            info!("🔥 Stopping Phoenix daemon (PID: {:?})...", pid);
            state.add_log(LogEntry {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                message: format!("Stopping Phoenix daemon (PID: {:?})", pid),
                source: "phoenix-api".to_string(),
                mode: None,
            });
            
            // Try to gracefully terminate the process
            match child.kill().await {
                Ok(_) => {
                    // Wait for the process to exit
                    match child.wait().await {
                        Ok(exit_status) => {
                            let success_msg = format!(
                                "Phoenix daemon stopped successfully (PID: {:?}, Exit: {})",
                                pid, exit_status
                            );
                            info!("✅ {}", success_msg);
                            state.add_log(LogEntry {
                                timestamp: Utc::now(),
                                level: "INFO".to_string(),
                                message: success_msg.clone(),
                                source: "phoenix-api".to_string(),
                                mode: None,
                            });
                            
                            Ok(ResponseJson(PhoenixStopResponse {
                                success: true,
                                message: success_msg,
                                pid,
                            }))
                        }
                        Err(e) => {
                            let warning_msg = format!(
                                "Phoenix daemon killed (PID: {:?}) but failed to wait for exit: {}",
                                pid, e
                            );
                            info!("⚠️ {}", warning_msg);
                            state.add_log(LogEntry {
                                timestamp: Utc::now(),
                                level: "WARN".to_string(),
                                message: warning_msg.clone(),
                                source: "phoenix-api".to_string(),
                                mode: None,
                            });
                            
                            Ok(ResponseJson(PhoenixStopResponse {
                                success: true,
                                message: warning_msg,
                                pid,
                            }))
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to stop Phoenix daemon (PID: {:?}): {}", pid, e);
                    info!("❌ {}", error_msg);
                    state.add_log(LogEntry {
                        timestamp: Utc::now(),
                        level: "ERROR".to_string(),
                        message: error_msg.clone(),
                        source: "phoenix-api".to_string(),
                        mode: None,
                    });
                    
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        None => {
            let msg = "Phoenix daemon is not currently running".to_string();
            info!("ℹ️ {}", msg);
            state.add_log(LogEntry {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                message: msg.clone(),
                source: "phoenix-api".to_string(),
                mode: None,
            });
            
            Ok(ResponseJson(PhoenixStopResponse {
                success: true,
                message: msg,
                pid: None,
            }))
        }
    }
}

/// API endpoint to detect existing Phoenix configuration
async fn detect_phoenix_config_api(State(state): State<AppState>) -> Result<ResponseJson<PhoenixStartResponse>, StatusCode> {
    info!("🔍 Attempting to detect existing Phoenix configuration...");
    
    // Check if Phoenix process is running in our state
    let is_running = {
        let process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        process_guard.is_some()
    };
    
    match get_existing_phoenix_config().await {
        Ok((url, password)) => {
            info!("✅ Found existing Phoenix configuration (running: {})", is_running);
            Ok(ResponseJson(PhoenixStartResponse {
                success: true,
                message: format!("Existing Phoenix configuration detected (running: {})", is_running),
                downloaded: false,
                pid: None,
                url: Some(url),
                password: Some(password),
                is_running: Some(is_running),
            }))
        }
        Err(e) => {
            info!("❌ Could not detect Phoenix configuration: {}", e);
            // If config doesn't exist but process is running, return minimal info
            if is_running {
                Ok(ResponseJson(PhoenixStartResponse {
                    success: true,
                    message: "Phoenix is running but configuration not yet available".to_string(),
                    downloaded: false,
                    pid: None,
                    url: Some(PHOENIX_DEFAULT_URL.to_string()),
                    password: None,
                    is_running: Some(true),
                }))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}

/// Start phoenixd process and return PID along with config info
async fn start_phoenixd_process(state: &AppState, path_config: &PathConfig) -> Result<(u32, String, String), String> {
    let phoenixd_binary = path_config.get_executable_path("phoenixd");
    
    if !phoenixd_binary.exists() {
        return Err(format!("Phoenixd binary not found at: {:?}", phoenixd_binary));
    }
    
    info!("🔥 Starting phoenixd from: {:?}", phoenixd_binary);
    
    // Set phoenixd working directory to app data directory to ensure it can write files
    let phoenixd_working_dir = path_config.data_dir.join("phoenixd");
    if let Err(e) = std::fs::create_dir_all(&phoenixd_working_dir) {
        info!("⚠️ Warning: Could not create phoenixd directory {:?}: {}", phoenixd_working_dir, e);
        info!("   Phoenixd will use current directory for data files");
    }
    
    let mut child = TokioCommand::new(&phoenixd_binary)
        .current_dir(&phoenixd_working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start phoenixd: {}", e))?;
    
    let pid = child.id().ok_or("Failed to get process ID")?;
    
    // Set up log readers for phoenixd (similar to existing wallet.rs implementation)
    if let Some(stdout) = child.stdout.take() {
        let reader = tokio::io::BufReader::new(stdout);
        let state_clone = state.clone();
        tokio::spawn(async move {
            crate::wallet::read_phoenixd_logs(reader, state_clone, "phoenixd-stdout").await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = tokio::io::BufReader::new(stderr);
        let state_clone = state.clone();
        tokio::spawn(async move {
            crate::wallet::read_phoenixd_stderr_logs(reader, state_clone, "phoenixd-stderr").await;
        });
    }
    
    // Store the phoenixd process
    {
        let mut process_guard = state.wallet_state.phoenixd_process.lock().unwrap();
        *process_guard = Some(child);
    }
    
    info!("✅ Phoenix daemon started with PID: {}", pid);
    info!("⏳ Waiting for Phoenix daemon to initialize and create config file...");
    
    // Wait for phoenix.conf to be created and get the password
    match wait_for_phoenix_conf_and_get_password().await {
        Ok(password) => {
            let default_url = PHOENIX_DEFAULT_URL.to_string();
            info!("✅ Phoenix configuration ready:");
            info!("   URL: {}", default_url);
            info!("   Password: {}***", &password[..std::cmp::min(4, password.len())]);
            
            Ok((pid, default_url, password))
        }
        Err(e) => {
            info!("⚠️ Could not get Phoenix password: {}", e);
            info!("   Phoenix daemon is running but configuration may need manual setup");
            // Still return success but with empty config
            Ok((pid, PHOENIX_DEFAULT_URL.to_string(), "".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_platform_and_arch() {
        let result = detect_platform_and_arch();
        assert!(result.is_ok(), "Should detect platform successfully");
        
        let (platform, arch) = result.unwrap();
        info!("Detected platform: {}-{}", platform, arch);
        
        // Verify it's one of the supported combinations
        match platform.as_str() {
            "macos" => assert!(arch == "x64" || arch == "arm64"),
            "linux" => assert!(arch == "x64" || arch == "arm64"),
            "windows" => assert_eq!(arch, "jvm"),
            _ => panic!("Unsupported platform: {}", platform),
        }
    }
    
    #[test]
    fn test_get_download_url() {
        let macos_x64_url = get_download_url("macos", "x64").unwrap();
        assert_eq!(
            macos_x64_url,
            format!("{}/v{}/phoenixd-{}-macos-x64.zip", PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION)
        );
        
        let windows_url = get_download_url("windows", "jvm").unwrap();
        assert_eq!(
            windows_url,
            format!("{}/v{}/phoenixd-{}-jvm.zip", PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION)
        );
        
        let linux_arm64_url = get_download_url("linux", "arm64").unwrap();
        assert_eq!(
            linux_arm64_url,
            format!("{}/v{}/phoenixd-{}-linux-arm64.zip", PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION)
        );
    }

    #[test]
    fn test_extraction_logic() {
        // Test that we correctly identify files that should be moved to bin root
        let test_cases = vec![
            ("phoenixd", true),
            ("phoenix-cli", true),
            ("phoenixd.exe", true), // Windows
            ("phoenix-cli.exe", true), // Windows
            ("README.md", false),
            ("lib/something.so", false),
            ("phoenixd-0.6.1-macos-arm64/phoenixd", true), // Should be extracted from nested path
            ("phoenixd-0.6.1-linux-x64/phoenix-cli", true), // Should be extracted from nested path
            ("phoenixd-0.6.1-windows/phoenixd.exe", true), // Windows from nested path
            ("phoenixd-0.6.1-jvm/phoenix-cli.exe", true), // Windows CLI from nested path
        ];
        
        for (filename, should_be_moved) in test_cases {
            let path = std::path::Path::new(filename);
            let actual_filename = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            
            // This matches the logic in extract_zip_blocking
            let is_phoenixd_executable = actual_filename == "phoenixd" || 
                                         actual_filename == "phoenix-cli" ||
                                         actual_filename == "phoenixd.exe" || 
                                         actual_filename == "phoenix-cli.exe";
            
            assert_eq!(
                is_phoenixd_executable, 
                should_be_moved,
                "File '{}' -> filename '{}', is_phoenixd_executable = {}, expected = {}", 
                filename, 
                actual_filename,
                is_phoenixd_executable, 
                should_be_moved
            );
        }
    }
}