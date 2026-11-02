use log::info;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tokio::task;
use zip::ZipArchive;

use axum::{
    extract::State, http::StatusCode, response::Json as ResponseJson, routing::post, Router,
};
use serde::Serialize;

use crate::{paths::PathConfig, state::AppState};

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

    info!(
        "🔥 Downloading Phoenix {} for {}-{}...",
        PHOENIX_VERSION, platform, arch
    );
    info!("📥 Download URL: {}", download_url);

    // Download the file
    let zip_data = download_file(&download_url).await?;

    // Extract to bin directory
    extract_phoenix_archive(&zip_data, &path_config.bin_dir, &platform).await?;

    let success_msg = format!(
        "✅ Phoenix {} successfully downloaded and extracted to {:?}",
        PHOENIX_VERSION, path_config.bin_dir
    );
    info!("{}", success_msg);
    Ok(success_msg)
}

/// Convenience function to download Phoenix using default PathConfig
pub async fn download_phoenix_default() -> Result<String, String> {
    let path_config =
        PathConfig::new().map_err(|e| format!("Failed to create PathConfig: {}", e))?;
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

    Ok(format!(
        "{}/v{}/{}",
        PHOENIX_BASE_URL, PHOENIX_VERSION, filename
    ))
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
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length();
    if let Some(size) = total_size {
        info!(
            "📦 Download size: {} bytes ({:.2} MB)",
            size,
            size as f64 / 1_048_576.0
        );
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
    platform: &str,
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
    task::spawn_blocking(move || extract_zip_blocking(&zip_data, &bin_dir, &platform))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

/// Blocking zip extraction function
fn extract_zip_blocking(zip_data: &[u8], bin_dir: &Path, platform: &str) -> Result<(), String> {
    let cursor = Cursor::new(zip_data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    info!(
        "📂 Extracting {} files from Phoenix archive...",
        archive.len()
    );

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
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
        let filename = original_path
            .file_name()
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
        let is_phoenixd_executable = filename == "phoenixd"
            || filename == "phoenix-cli"
            || (platform == "windows"
                && (filename == "phoenixd.exe" || filename == "phoenix-cli.exe"));

        if platform != "windows" && is_phoenixd_executable {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = outfile
                    .metadata()
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
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;

    let phoenix_conf_path = home_dir.join(".phoenix").join("phoenix.conf");

    if !phoenix_conf_path.exists() {
        return Err(format!(
            "Phoenix config file not found at: {:?}",
            phoenix_conf_path
        ));
    }

    let conf_content = std::fs::read_to_string(&phoenix_conf_path)
        .map_err(|e| format!("Failed to read phoenix.conf: {}", e))?;

    // Parse the config file line by line looking for http-password=
    for line in conf_content.lines() {
        let line = line.trim();
        if line.starts_with("http-password=") {
            // Split on '=' and take everything after the first '='
            if let Some(password) = line.strip_prefix("http-password=") {
                if !password.trim().is_empty() {
                    return Ok(password.to_string());
                }
            }
        }
    }

    Err("http-password not found in phoenix.conf".to_string())
}

pub(crate) fn has_existing_phoenix_credentials() -> bool {
    get_phoenixd_password().is_ok()
}
/// Readiness is an authenticated read, never merely a config file or PID.
async fn phoenix_ready(url: &str, password: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    match client
        .get(format!("{}/getinfo", url.trim_end_matches('/')))
        .basic_auth("", Some(password))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => match response.text().await {
            Ok(body) => serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| {
                    value
                        .get("nodeId")
                        .and_then(|id| id.as_str())
                        .map(|id| id.len() == 66)
                })
                .unwrap_or(false),
            Err(_) => false,
        },
        Ok(_) => false,
        Err(_) => false,
    }
}
/// Boot and manual start share the same process owner and serialized lifecycle.
pub async fn start_phoenix_with_config(state: &AppState) -> Result<PhoenixStartResponse, String> {
    let _lifecycle = state.wallet_state.phoenixd_lifecycle.lock().await;
    if let Ok((url, password)) = get_existing_phoenix_config().await {
        if phoenix_ready(&url, &password).await {
            refresh_lightning_node(state).await;
            return Ok(phoenix_response(
                crate::wallet::phoenixd_pid(state)?,
                url,
                password,
                false,
                true,
            ));
        }
    }
    if crate::wallet::phoenixd_pid(state)?.is_none()
        && tokio::net::TcpStream::connect("127.0.0.1:9740")
            .await
            .is_ok()
    {
        return Err(
            "Phoenix endpoint is already occupied but not ready with the configured credentials"
                .into(),
        );
    }
    let path_config = &state.path_config;
    let downloaded = !path_config.get_executable_path("phoenixd").exists();
    if downloaded {
        download_phoenix(path_config).await?;
    }
    crate::wallet::spawn_phoenixd(state)?;
    for _ in 0..30 {
        if crate::wallet::phoenixd_pid(state)?.is_none() {
            return Err("Phoenix exited before its API became ready".into());
        }
        if let Ok((url, password)) = get_existing_phoenix_config().await {
            if phoenix_ready(&url, &password).await {
                refresh_lightning_node(state).await;
                return Ok(phoenix_response(
                    crate::wallet::phoenixd_pid(state)?,
                    url,
                    password,
                    downloaded,
                    true,
                ));
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Err("Phoenix is still starting; its authenticated API did not become ready in time".into())
}
async fn refresh_lightning_node(state: &AppState) {
    // Reload only the user's existing default; Start never changes selection.
    if let Ok(node) =
        crate::lightning::LightningNode::from_torrc(state.path_config.get_torrc_path(None)).await
    {
        state.set_lightning_node(node);
    }
}
fn phoenix_response(
    pid: Option<u32>,
    url: String,
    password: String,
    downloaded: bool,
    running: bool,
) -> PhoenixStartResponse {
    PhoenixStartResponse {
        success: true,
        message: if running {
            "Phoenix wallet ready".into()
        } else {
            "Phoenix configuration detected; wallet is stopped".into()
        },
        downloaded,
        pid,
        url: Some(url),
        password: Some(password),
        is_running: Some(running),
    }
}
pub async fn detect_phoenix_with_config(state: &AppState) -> Result<PhoenixStartResponse, String> {
    let (url, password) = get_existing_phoenix_config().await?;
    let pid = crate::wallet::phoenixd_pid(state)?;
    let running = pid.is_some() || phoenix_ready(&url, &password).await;
    Ok(phoenix_response(pid, url, password, false, running))
}

/// API endpoint response for Phoenix start operation
#[derive(Serialize)]
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
        .route(
            "/api/phoenix/detect-config",
            post(detect_phoenix_config_api),
        )
}

/// API endpoint to start Phoenix daemon
/// This will check if phoenixd binary exists, download it if needed, and start the process
async fn start_phoenix_api(
    State(state): State<AppState>,
) -> Result<ResponseJson<PhoenixStartResponse>, StatusCode> {
    start_phoenix_with_config(&state)
        .await
        .map(ResponseJson)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
async fn stop_phoenix_api(
    State(state): State<AppState>,
) -> Result<ResponseJson<PhoenixStopResponse>, StatusCode> {
    let pid = crate::wallet::phoenixd_pid(&state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if pid.is_some() {
        crate::wallet::stop_phoenixd(state)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(ResponseJson(PhoenixStopResponse {
        success: pid.is_some(),
        message: if pid.is_some() {
            "Managed Phoenix wallet stopped".into()
        } else {
            "No managed Phoenix process; an external wallet was not stopped".into()
        },
        pid,
    }))
}
async fn detect_phoenix_config_api(
    State(state): State<AppState>,
) -> Result<ResponseJson<PhoenixStartResponse>, StatusCode> {
    detect_phoenix_with_config(&state)
        .await
        .map(ResponseJson)
        .map_err(|_| StatusCode::NOT_FOUND)
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
            format!(
                "{}/v{}/phoenixd-{}-macos-x64.zip",
                PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION
            )
        );

        let windows_url = get_download_url("windows", "jvm").unwrap();
        assert_eq!(
            windows_url,
            format!(
                "{}/v{}/phoenixd-{}-jvm.zip",
                PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION
            )
        );

        let linux_arm64_url = get_download_url("linux", "arm64").unwrap();
        assert_eq!(
            linux_arm64_url,
            format!(
                "{}/v{}/phoenixd-{}-linux-arm64.zip",
                PHOENIX_BASE_URL, PHOENIX_VERSION, PHOENIX_VERSION
            )
        );
    }

    #[test]
    fn test_extraction_logic() {
        // Test that we correctly identify files that should be moved to bin root
        let test_cases = vec![
            ("phoenixd", true),
            ("phoenix-cli", true),
            ("phoenixd.exe", true),    // Windows
            ("phoenix-cli.exe", true), // Windows
            ("README.md", false),
            ("lib/something.so", false),
            ("phoenixd-0.6.1-macos-arm64/phoenixd", true), // Should be extracted from nested path
            ("phoenixd-0.6.1-linux-x64/phoenix-cli", true), // Should be extracted from nested path
            ("phoenixd-0.6.1-windows/phoenixd.exe", true), // Windows from nested path
            ("phoenixd-0.6.1-jvm/phoenix-cli.exe", true),  // Windows CLI from nested path
        ];

        for (filename, should_be_moved) in test_cases {
            let path = std::path::Path::new(filename);
            let actual_filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // This matches the logic in extract_zip_blocking
            let is_phoenixd_executable = actual_filename == "phoenixd"
                || actual_filename == "phoenix-cli"
                || actual_filename == "phoenixd.exe"
                || actual_filename == "phoenix-cli.exe";

            assert_eq!(
                is_phoenixd_executable, should_be_moved,
                "File '{}' -> filename '{}', is_phoenixd_executable = {}, expected = {}",
                filename, actual_filename, is_phoenixd_executable, should_be_moved
            );
        }
    }
}
