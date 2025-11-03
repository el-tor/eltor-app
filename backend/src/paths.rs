use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use log::info;
use crate::ip;

/// Central path configuration for the application
#[derive(Debug, Clone)]
pub struct PathConfig {
    pub bin_dir: PathBuf,
    pub data_dir: PathBuf,
    pub app_data_dir: Option<PathBuf>,
}

impl PathConfig {
    /// Create a new PathConfig by detecting the current environment
    pub fn new() -> Result<Self, String> {
        let (bin_dir, data_dir, app_data_dir) = detect_paths()?;
        
        Ok(PathConfig {
            bin_dir,
            data_dir,
            app_data_dir,
        })
    }

    /// Create a PathConfig with custom override paths
    pub fn with_overrides(
        bin_dir_override: Option<PathBuf>,
        data_dir_override: Option<PathBuf>,
    ) -> Result<Self, String> {
        if let (Some(bin_dir), Some(data_dir)) = (bin_dir_override, data_dir_override) {
            Ok(PathConfig {
                bin_dir,
                data_dir: data_dir.clone(),
                app_data_dir: Some(data_dir),
            })
        } else {
            Self::new()
        }
    }

    /// Create a PathConfig specifically for Tauri applications with a resource directory
    /// This is the main entry point for Tauri apps
    pub fn for_tauri_with_resource_dir(resource_dir: PathBuf) -> Result<Self, String> {
        let app_data_dir = get_app_data_dir()?;
        
        Ok(PathConfig {
            bin_dir: resource_dir,
            data_dir: app_data_dir.clone(),
            app_data_dir: Some(app_data_dir),
        })
    }

    /// Get the path to a torrc file
    pub fn get_torrc_path(&self, filename: Option<&str>) -> PathBuf {
        let filename = filename.unwrap_or("torrc");
        self.data_dir.join(filename)
    }

    /// Get the path to torrc.relay file
    pub fn get_torrc_relay_path(&self) -> PathBuf {
        self.data_dir.join("torrc.relay")
    }

    /// Get the path to an executable
    pub fn get_executable_path(&self, name: &str) -> PathBuf {
        #[cfg(windows)]
        let name = format!("{}.exe", name);
        
        self.bin_dir.join(name)
    }

    /// Ensure torrc files exist, creating from templates if needed
    pub fn ensure_torrc_files(&self) -> Result<(), String> {
        // Create data directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&self.data_dir) {
            info!("⚠️ Warning: Failed to create data directory {:?}: {}", self.data_dir, e);
            return Err(format!("Failed to create data directory: {}", e));
        }

        // Create Tor subdirectories that will be needed for logs and other data
        let tor_data_dir = self.data_dir.join("tor_data");
        let client_dir = tor_data_dir.join("client");
        let relay_dir = tor_data_dir.join("relay");
        
        if let Err(e) = fs::create_dir_all(&client_dir) {
            info!("⚠️ Warning: Failed to create client directory {:?}: {}", client_dir, e);
            return Err(format!("Failed to create client directory: {}", e));
        }
        
        if let Err(e) = fs::create_dir_all(&relay_dir) {
            info!("⚠️ Warning: Failed to create relay directory {:?}: {}", relay_dir, e);
            return Err(format!("Failed to create relay directory: {}", e));
        }

        let torrc_path = self.get_torrc_path(None);
        let torrc_relay_path = self.get_torrc_relay_path();

        if !torrc_path.exists() {
            info!("path.rs torrc file does not exist. Creating torrc file at: {:?}", torrc_path);
            create_torrc_from_template(&torrc_path, &self.bin_dir)?;
        }

        if !torrc_relay_path.exists() {
            info!("path.rs torrc.relay file does not exist. Creating torrc.relay file at: {:?}", torrc_relay_path);
            create_torrc_relay_from_template(&torrc_relay_path, &self.bin_dir)?;
        }

        Ok(())
    }

    /// Get the data directory for a specific torrc file
    pub fn get_torrc_data_dir(&self, torrc_filename: &str) -> PathBuf {
        // For backward compatibility with existing code that uses "./bin/{torrc_filename}"
        if let Some(filename) = torrc_filename.strip_prefix("data/") {
            self.data_dir.join(filename)
        } else {
            self.data_dir.join(torrc_filename)
        }
    }

    /// Get paths in the format expected by existing port parsing code
    pub fn get_torrc_path_for_ports(&self, torrc_filename: &str) -> String {
        if torrc_filename.starts_with("data/") {
            format!("./bin/{}", torrc_filename)
        } else {
            format!("./bin/data/{}", torrc_filename)
        }
    }
}

/// Detect paths based on current environment
pub fn detect_paths() -> Result<(PathBuf, PathBuf, Option<PathBuf>), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    info!("🔍 PathConfig Debug - detect_paths():");
    info!("   Current dir: {:?}", current_dir);
    if let Ok(exe_path) = env::current_exe() {
        info!("   Current exe: {:?}", exe_path);
    }
    
    // Check environment variable override first
    if let Ok(override_path) = env::var("ELTOR_BIN_DIR") {
        info!("   Using ELTOR_BIN_DIR override: {}", override_path);
        let bin_dir = PathBuf::from(override_path);
        let data_dir = if let Ok(data_override) = env::var("ELTOR_DATA_DIR") {
            PathBuf::from(data_override)
        } else {
            bin_dir.join("data")
        };
        return Ok((bin_dir, data_dir, None));
    }

    // Docker environment detection
    if env::var("ELTOR_DOCKER_ENV").is_ok() || 
       Path::new("/home/user/code/eltor-app").exists() {
        info!("   Detected Docker environment");
        let base = PathBuf::from("/home/user/code/eltor-app/backend");
        return Ok((base.join("bin"), base.join("bin/data"), None));
    }

    // Check if we're in a Tauri bundled app context
    if let Ok(exe_path) = env::current_exe() {
        let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
        
        // Check if we're in a macOS app bundle
        if exe_path.to_string_lossy().contains(".app/Contents/MacOS/") {
            info!("   Detected macOS app bundle");
            let app_data_dir = get_app_data_dir()?;
            
            // In a macOS app bundle, resources are in ../Resources/
            // exe_path is typically: App.app/Contents/MacOS/app-name
            // So we need: App.app/Contents/Resources/
            if let Some(contents_dir) = exe_dir.parent() {
                let resources_dir = contents_dir.join("Resources");
                info!("   Checking Resources directory: {:?}", resources_dir);
                
                if resources_dir.exists() {
                    info!("   ✅ Found Resources directory");
                    
                    // Check for Tauri's _up_ structure (when resources use relative paths like ../../)
                    let tauri_bin_dir = resources_dir.join("_up_").join("_up_").join("backend").join("bin");
                    if tauri_bin_dir.exists() {
                        info!("   ✅ Found Tauri _up_ structure at: {:?}", tauri_bin_dir);
                        
                        // List files for debugging
                        if let Ok(entries) = fs::read_dir(&tauri_bin_dir) {
                            info!("   📁 Files in Tauri bin directory:");
                            for entry in entries.flatten() {
                                info!("      - {:?}", entry.file_name());
                            }
                        }
                        
                        return Ok((tauri_bin_dir, app_data_dir.clone(), Some(app_data_dir)));
                    }
                    
                    // Otherwise check if files are directly in Resources
                    if let Ok(entries) = fs::read_dir(&resources_dir) {
                        info!("   📁 Files in Resources:");
                        for entry in entries.flatten() {
                            info!("      - {:?}", entry.file_name());
                        }
                    }
                    
                    return Ok((resources_dir, app_data_dir.clone(), Some(app_data_dir)));
                } else {
                    info!("   ⚠️  Resources directory not found at: {:?}", resources_dir);
                }
            }
        }
        
        // Check if we're in a general bundled context (resources in same directory)
        if exe_dir.join("torrc.template").exists() || exe_dir.join("phoenixd").exists() {
            info!("   Detected bundled context (resources in exe dir)");
            let app_data_dir = get_app_data_dir()?;
            return Ok((exe_dir.to_path_buf(), app_data_dir.clone(), Some(app_data_dir)));
        }
    }

    // Development environment detection
    info!("   Using development environment paths");
    let backend_dir = find_backend_dir(&current_dir)?;
    let bin_dir = backend_dir.join("bin");
    let data_dir = bin_dir.join("data");

    // For Tauri apps, also set app data directory
    let app_data_dir = if is_tauri_context() {
        Some(get_app_data_dir()?)
    } else {
        None
    };

    Ok((bin_dir, data_dir, app_data_dir))
}

/// Find the backend directory from current working directory
fn find_backend_dir(current_dir: &Path) -> Result<PathBuf, String> {
    if current_dir.ends_with("backend") {
        Ok(current_dir.to_path_buf())
    } else if current_dir.join("backend").exists() {
        Ok(current_dir.join("backend"))
    } else if current_dir.ends_with("src-tauri") {
        // Safely navigate parent directories with error handling
        current_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("backend"))
            .ok_or_else(|| "Cannot navigate to backend directory from src-tauri".to_string())
    } else if current_dir.ends_with("frontend") {
        current_dir
            .parent()
            .map(|p| p.join("backend"))
            .ok_or_else(|| "Cannot navigate to backend directory from frontend".to_string())
    } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let path = Path::new(&manifest_dir);
        if path.ends_with("backend") {
            Ok(path.to_path_buf())
        } else {
            Ok(path.join("backend"))
        }
    } else {
        Err("Could not locate backend directory".to_string())
    }
}

pub fn is_tauri_context() -> bool {
    env::var("ELTOR_TAURI_MODE").is_ok()
}

fn get_app_data_dir() -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or("Failed to get data directory")?
        .join("eltor");

    // Try to create app data directory with fallback to temp directory for DMG compatibility
    match fs::create_dir_all(&app_data_dir) {
        Ok(_) => Ok(app_data_dir),
        Err(e) => {
            info!("⚠️ Warning: Could not create app data directory {:?}: {}", app_data_dir, e);
            info!("   This might be due to running from a read-only DMG. Using temp directory fallback...");
            
            // Fallback to temporary directory
            let temp_dir = std::env::temp_dir().join("eltor");
            fs::create_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to create temp data directory: {}", e))?;
            
            info!("✅ Using temporary directory for DMG compatibility: {:?}", temp_dir);
            Ok(temp_dir)
        }
    }
}

fn create_torrc_from_template(torrc_path: &Path, bin_dir: &Path) -> Result<(), String> {
    let template_path = bin_dir.join("torrc.template");
    let template_content = if template_path.exists() {
        fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read torrc template: {}", e))?
    } else {
        // Fallback to embedded template - will be defined in lib.rs
        include_str!("../bin/torrc.template").to_string()
    };

    let content = substitute_torrc_variables(template_content)?;
    fs::write(torrc_path, content)
        .map_err(|e| format!("Failed to write torrc file: {}", e))?;

    info!("✅ Created torrc file at: {:?}", torrc_path);
    Ok(())
}

fn create_torrc_relay_from_template(torrc_relay_path: &Path, bin_dir: &Path) -> Result<(), String> {
    let template_path = bin_dir.join("torrc.relay.template");
    let template_content = if template_path.exists() {
        fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read torrc.relay template: {}", e))?
    } else {
        // Fallback to embedded template - will be defined in lib.rs
        include_str!("../bin/torrc.relay.template").to_string()
    };

    let content = substitute_torrc_relay_variables(template_content)?;
    fs::write(torrc_relay_path, content)
        .map_err(|e| format!("Failed to write torrc.relay file: {}", e))?;

    info!("✅ Created torrc.relay file at: {:?}", torrc_relay_path);
    Ok(())
}

/// Substitute variables in torrc template
fn substitute_torrc_variables(mut content: String) -> Result<String, String> {
    use rand::Rng;

    // Generate random nickname if not provided (12-19 chars, alphanumeric only)
    let mut rng = rand::thread_rng();
    let nickname_length = rng.gen_range(12..=19);
    let random_nickname: String = (0..nickname_length)
        .map(|_| {
            let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            charset[rng.gen_range(0..charset.len())] as char
        })
        .collect();

    // Get the appropriate app data directory for Tor data
    let default_tor_data_dir = get_app_data_dir()
        .map(|dir| {
            let tor_data_path = dir.join("tor_data");
            // Ensure the directory exists and is writable
            if let Err(e) = fs::create_dir_all(&tor_data_path) {
                info!("⚠️ Warning: Could not create tor data directory {:?}: {}", tor_data_path, e);
                return "/tmp/tor".to_string();
            }
            tor_data_path.to_string_lossy().to_string()
        })
        .unwrap_or_else(|_| "/tmp/tor".to_string());

    // Use environment variables or defaults
    content = content.replace(
        "$APP_ELTOR_TOR_NICKNAME",
        &env::var("APP_ELTOR_TOR_NICKNAME")
            .unwrap_or(random_nickname),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_DATA_DIRECTORY",
        &env::var("APP_ELTOR_TOR_DATA_DIRECTORY")
            .unwrap_or_else(|_| default_tor_data_dir.clone()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_SOCKS_PORT",
        &env::var("APP_ELTOR_TOR_SOCKS_PORT").unwrap_or_else(|_| "0.0.0.0:18058".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_CONTROL_PORT",
        &env::var("APP_ELTOR_TOR_CONTROL_PORT").unwrap_or_else(|_| "9992".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD",
        &env::var("APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD")
            .unwrap_or_else(|_| "16:281EC5644A4F548A60D50A0DD4DF835FFD50EDED062FD270D7269943DA".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_CLIENT_ADDRESS",
        &env::var("APP_ELTOR_TOR_CLIENT_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_PAYMENT_CIRCUIT_MAX_FEE",
        &env::var("APP_ELTOR_TOR_PAYMENT_CIRCUIT_MAX_FEE").unwrap_or_else(|_| "1000".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_LN_CONFIG",
        &env::var("APP_ELTOR_LN_CONFIG").unwrap_or_else(|_| "".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_ADDITIONAL_DIR_AUTHORITY",
        &env::var("APP_ELTOR_TOR_ADDITIONAL_DIR_AUTHORITY").unwrap_or_else(|_| "".to_string()),
    );

    Ok(content)
}

/// Substitute variables in torrc.relay template
fn substitute_torrc_relay_variables(mut content: String) -> Result<String, String> {
    use rand::Rng;

    // Generate random relay nickname if not provided (12-19 chars, alphanumeric only)
    let mut rng = rand::thread_rng();
    let nickname_length = rng.gen_range(12..=19);
    let random_relay_nickname: String = (0..nickname_length)
        .map(|_| {
            let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            charset[rng.gen_range(0..charset.len())] as char
        })
        .collect();

    // Get the appropriate app data directory for Tor relay data
    let default_tor_relay_data_dir = get_app_data_dir()
        .map(|dir| dir.join("tor_data").join("relay").to_string_lossy().to_string())
        .unwrap_or_else(|_| "/tmp/tor-relay".to_string());

    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_NICKNAME",
        &env::var("APP_ELTOR_TOR_RELAY_NICKNAME").unwrap_or(random_relay_nickname),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_DATA_DIRECTORY",
        &env::var("APP_ELTOR_TOR_RELAY_DATA_DIRECTORY")
            .unwrap_or_else(|_| default_tor_relay_data_dir.clone()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_OR_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_OR_PORT").unwrap_or_else(|_| "9996".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_CONTROL_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_CONTROL_PORT").unwrap_or_else(|_| "7781".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_SOCKS_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_SOCKS_PORT").unwrap_or_else(|_| "0.0.0.0:18057".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD",
        &env::var("APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD")
            .unwrap_or_else(|_| "16:281EC5644A4F548A60D50A0DD4DF835FFD50EDED062FD270D7269943DA".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_ADDRESS",
        &env::var("APP_ELTOR_TOR_RELAY_ADDRESS").unwrap_or_else(|_| ip::get_public_ip_blocking()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_CONTACT",
        &env::var("APP_ELTOR_TOR_RELAY_CONTACT").unwrap_or_else(|_| "eltor@example.com".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_SANDBOX",
        &env::var("APP_ELTOR_TOR_RELAY_SANDBOX").unwrap_or_else(|_| "1".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_EXIT_RELAY",
        &env::var("APP_ELTOR_TOR_EXIT_RELAY").unwrap_or_else(|_| "0".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_PAYMENT_RATE_MSATS",
        &env::var("APP_ELTOR_TOR_RELAY_PAYMENT_RATE_MSATS").unwrap_or_else(|_| "1000".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL",
        &env::var("APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL").unwrap_or_else(|_| "300".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL_ROUNDS",
        &env::var("APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL_ROUNDS").unwrap_or_else(|_| "10".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_PAYMENT_CIRCUIT_MAX_FEE",
        &env::var("APP_ELTOR_TOR_RELAY_PAYMENT_CIRCUIT_MAX_FEE").unwrap_or_else(|_| "1000".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_LN_BOLT12",
        &env::var("APP_ELTOR_LN_BOLT12").unwrap_or_else(|_| "".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_LN_CONFIG",
        &env::var("APP_ELTOR_LN_CONFIG")
            .unwrap_or_else(|_| "type=phoenixd url=http://localhost:9740 password=password default=true".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_ADDITIONAL_DIR_AUTHORITY",
        &env::var("APP_ELTOR_TOR_RELAY_ADDITIONAL_DIR_AUTHORITY").unwrap_or_else(|_| "".to_string()),
    );

    Ok(content)
}
