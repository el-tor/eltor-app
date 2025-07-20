use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use rand::Rng;

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
                data_dir,
                app_data_dir: None,
            })
        } else {
            Self::new()
        }
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
        fs::create_dir_all(&self.data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let torrc_path = self.get_torrc_path(None);
        let torrc_relay_path = self.get_torrc_relay_path();

        if !torrc_path.exists() {
            create_torrc_from_template(&torrc_path, &self.bin_dir)?;
        }

        if !torrc_relay_path.exists() {
            create_torrc_relay_from_template(&torrc_relay_path, &self.bin_dir)?;
        }

        Ok(())
    }

    /// Get the data directory for a specific torrc file
    pub fn get_torrc_data_dir(&self, torrc_filename: &str) -> PathBuf {
        // For backward compatibility with existing code that uses "./bin/{torrc_filename}"
        if torrc_filename.starts_with("data/") {
            self.data_dir.join(&torrc_filename[5..])
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
fn detect_paths() -> Result<(PathBuf, PathBuf, Option<PathBuf>), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // Check environment variable override first
    if let Ok(override_path) = env::var("ELTOR_BIN_DIR") {
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
        let base = PathBuf::from("/home/user/code/eltor-app/backend");
        return Ok((base.join("bin"), base.join("bin/data"), None));
    }

    // Development environment detection
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
        Ok(current_dir.parent().unwrap().parent().unwrap().join("backend"))
    } else if current_dir.ends_with("frontend") {
        Ok(current_dir.parent().unwrap().join("backend"))
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

fn is_tauri_context() -> bool {
    env::var("TAURI_ENV").is_ok() || 
    env::current_exe()
        .map(|exe| exe.to_string_lossy().contains("tauri"))
        .unwrap_or(false)
}

fn get_app_data_dir() -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or("Failed to get data directory")?
        .join("eltor");

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir)
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

    println!("✅ Created torrc file at: {:?}", torrc_path);
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

    println!("✅ Created torrc.relay file at: {:?}", torrc_relay_path);
    Ok(())
}

/// Substitute variables in torrc template
fn substitute_torrc_variables(mut content: String) -> Result<String, String> {
    use rand::Rng;

    // Generate random nickname if not provided
    let random_nickname: String = (0..12)
        .map(|_| {
            let mut rng = rand::thread_rng();
            rng.gen_range(b'a'..=b'z') as char
        })
        .collect();

    // Use environment variables or defaults
    content = content.replace(
        "$APP_ELTOR_TOR_NICKNAME",
        &env::var("APP_ELTOR_TOR_NICKNAME")
            .unwrap_or_else(|_| format!("eltor-{}", random_nickname)),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_DATA_DIRECTORY",
        &env::var("APP_ELTOR_TOR_DATA_DIRECTORY")
            .unwrap_or_else(|_| "/tmp/tor".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_SOCKS_PORT",
        &env::var("APP_ELTOR_TOR_SOCKS_PORT").unwrap_or_else(|_| "9050".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_CONTROL_PORT",
        &env::var("APP_ELTOR_TOR_CONTROL_PORT").unwrap_or_else(|_| "9051".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD",
        &env::var("APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD")
            .unwrap_or_else(|_| "16:872860B76453A77D60CA2BB8C1A7042072093276A3D701AD684053EC4C".to_string()),
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
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_NICKNAME",
        &env::var("APP_ELTOR_TOR_RELAY_NICKNAME").unwrap_or_else(|_| "eltor-relay".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_DATA_DIRECTORY",
        &env::var("APP_ELTOR_TOR_RELAY_DATA_DIRECTORY")
            .unwrap_or_else(|_| "/tmp/tor-relay".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_OR_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_OR_PORT").unwrap_or_else(|_| "9001".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_CONTROL_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_CONTROL_PORT").unwrap_or_else(|_| "9052".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_SOCKS_PORT",
        &env::var("APP_ELTOR_TOR_RELAY_SOCKS_PORT").unwrap_or_else(|_| "9150".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD",
        &env::var("APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD")
            .unwrap_or_else(|_| "16:872860B76453A77D60CA2BB8C1A7042072093276A3D701AD684053EC4C".to_string()),
    );
    
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_ADDRESS",
        &env::var("APP_ELTOR_TOR_RELAY_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string()),
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
