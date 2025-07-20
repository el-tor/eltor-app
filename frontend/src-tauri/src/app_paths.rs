use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};

// TODO depreceiate for the backend/paths.rs

/// Get the app data directory where we store configuration files
pub fn get_app_data_dir() -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or("Failed to get data directory")?
        .join("eltor");

    // Create the directory if it doesn't exist
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    Ok(app_data_dir)
}

/// Get the bin directory where executables are stored (either bundled resources or dev path)
pub fn get_bin_dir() -> Result<PathBuf, String> {
    // Try development paths first
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let backend_dir = if current_dir.ends_with("src-tauri") {
        // We're in src-tauri, go up to find backend
        current_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("backend")
    } else if current_dir.join("backend").exists() {
        // We're in the root project directory
        current_dir.join("backend")
    } else {
        // Try to find backend relative to executable location
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent().unwrap();

        // For bundled apps, resources are typically in the same directory as the executable
        // or in a Resources subdirectory
        let candidates = vec![
            exe_dir.to_path_buf(),     // Same directory as executable
            exe_dir.join("Resources"), // macOS app bundle
            exe_dir.join("backend"),
            exe_dir.join("../backend"),
            exe_dir.join("../../backend"),
        ];

        candidates
            .into_iter()
            .find(|path| path.join("eltord").exists() || path.join("eltord.exe").exists())
            .unwrap_or_else(|| exe_dir.to_path_buf())
    };

    let bin_dir = backend_dir.join("bin");

    // If bin directory doesn't exist, try the backend directory itself
    if !bin_dir.exists()
        && (backend_dir.join("eltord").exists() || backend_dir.join("eltord.exe").exists())
    {
        return Ok(backend_dir);
    }

    Ok(bin_dir)
}

/// Get the torrc file path, creating it from template if needed
pub fn get_torrc_path() -> Result<PathBuf, String> {
    // First check if we're in development mode and the backend/bin/data directory exists
    if let Ok(dev_torrc_path) = get_dev_torrc_path() {
        // Ensure both torrc files exist in development directory
        ensure_torrc_files_exist(dev_torrc_path.parent().unwrap())?;
        return Ok(dev_torrc_path);
    }

    // Fallback to app data directory for bundled apps
    let app_data_dir = get_app_data_dir()?;
    let torrc_path = app_data_dir.join("torrc");

    // Ensure both torrc files exist in app data directory
    ensure_torrc_files_exist(&app_data_dir)?;

    Ok(torrc_path)
}

/// Get the torrc.relay file path
pub fn get_torrc_relay_path() -> Result<PathBuf, String> {
    // First check if we're in development mode and the backend/bin/data directory exists
    if let Ok(dev_torrc_path) = get_dev_torrc_path() {
        let data_dir = dev_torrc_path.parent().unwrap();
        ensure_torrc_files_exist(data_dir)?;
        return Ok(data_dir.join("torrc.relay"));
    }

    // Fallback to app data directory for bundled apps
    let app_data_dir = get_app_data_dir()?;
    ensure_torrc_files_exist(&app_data_dir)?;

    Ok(app_data_dir.join("torrc.relay"))
}

/// Initialize torrc files at app startup - call this early in your app initialization
pub fn initialize_torrc_files() -> Result<(), String> {
    println!("🔧 Initializing torrc files...");

    // Try to initialize in development directory first
    if let Ok(dev_torrc_path) = get_dev_torrc_path() {
        let data_dir = dev_torrc_path.parent().unwrap();
        ensure_torrc_files_exist(data_dir)?;
        println!(
            "✅ Initialized torrc files in development directory: {:?}",
            data_dir
        );
        return Ok(());
    }

    // Fallback to app data directory
    let app_data_dir = get_app_data_dir()?;
    ensure_torrc_files_exist(&app_data_dir)?;
    println!(
        "✅ Initialized torrc files in app data directory: {:?}",
        app_data_dir
    );

    Ok(())
}

/// Try to get the development torrc path (backend/bin/data/torrc)
fn get_dev_torrc_path() -> Result<PathBuf, String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let backend_dir = if current_dir.ends_with("src-tauri") {
        // We're in src-tauri, go up to find backend
        current_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("backend")
    } else if current_dir.join("backend").exists() {
        // We're in the root project directory
        current_dir.join("backend")
    } else {
        return Err("Not in development environment".to_string());
    };

    let dev_data_dir = backend_dir.join("bin").join("data");
    let torrc_path = dev_data_dir.join("torrc");

    // Check if we can create the development data directory
    if !dev_data_dir.exists() {
        fs::create_dir_all(&dev_data_dir)
            .map_err(|e| format!("Cannot create dev data directory: {}", e))?;
    }

    Ok(torrc_path)
}

/// Ensure both torrc and torrc.relay files exist in the given directory
fn ensure_torrc_files_exist(data_dir: &Path) -> Result<(), String> {
    let torrc_path = data_dir.join("torrc");
    let torrc_relay_path = data_dir.join("torrc.relay");

    // Create torrc file if it doesn't exist
    if !torrc_path.exists() {
        create_torrc_from_template(&torrc_path)?;
    }

    // Create torrc.relay file if it doesn't exist
    if !torrc_relay_path.exists() {
        create_torrc_relay_from_template(&torrc_relay_path)?;
    }

    Ok(())
}

/// Create torrc file from template
fn create_torrc_from_template(torrc_path: &Path) -> Result<(), String> {
    // Get the template content
    let template_content = get_torrc_template()?;

    // Create data directory if it doesn't exist
    if let Some(parent) = torrc_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create torrc directory: {}", e))?;
    }

    // Set default environment variables for the template
    let content = substitute_torrc_variables(template_content)?;

    // Write the torrc file
    fs::write(torrc_path, content).map_err(|e| format!("Failed to write torrc file: {}", e))?;

    println!("✅ Created torrc file at: {:?}", torrc_path);
    Ok(())
}

/// Create torrc.relay file from template
fn create_torrc_relay_from_template(torrc_relay_path: &Path) -> Result<(), String> {
    // Get the template content
    let template_content = get_torrc_relay_template()?;

    // Create data directory if it doesn't exist
    if let Some(parent) = torrc_relay_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create torrc.relay directory: {}", e))?;
    }

    // Set default environment variables for the template
    let content = substitute_torrc_relay_variables(template_content)?;

    // Write the torrc.relay file
    fs::write(torrc_relay_path, content)
        .map_err(|e| format!("Failed to write torrc.relay file: {}", e))?;

    println!("✅ Created torrc.relay file at: {:?}", torrc_relay_path);
    Ok(())
}

/// Substitute variables in torrc template
fn substitute_torrc_variables(mut content: String) -> Result<String, String> {
    let app_data_dir = get_app_data_dir()?;

    let random_nickname: String = (0..12)
        .map(|_| rand::rng().gen_range(b'a'..=b'z') as char)
        .collect();
    content = content.replace(
        "$APP_ELTOR_TOR_NICKNAME",
        &format!("eltor-{}", random_nickname),
    );
    content = content.replace(
        "$APP_ELTOR_TOR_DATA_DIRECTORY",
        &app_data_dir.join("tor_data").to_string_lossy(),
    );
    content = content.replace("$APP_ELTOR_TOR_SOCKS_PORT", "9050");
    content = content.replace("$APP_ELTOR_TOR_CONTROL_PORT", "9051");
    content = content.replace(
        "$APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD",
        "16:872860B76453A77D60CA2BB8C1A7042072093276A3D701AD684053EC4C",
    );
    content = content.replace("$APP_ELTOR_TOR_CLIENT_ADDRESS", "127.0.0.1");
    content = content.replace("$APP_ELTOR_TOR_PAYMENT_CIRCUIT_MAX_FEE", "1000");
    content = content.replace("$APP_ELTOR_LN_CONFIG", "");
    content = content.replace("$APP_ELTOR_TOR_ADDITIONAL_DIR_AUTHORITY", "");

    Ok(content)
}

/// Substitute variables in torrc.relay template
fn substitute_torrc_relay_variables(mut content: String) -> Result<String, String> {
    let app_data_dir = get_app_data_dir()?;

    content = content.replace("$APP_ELTOR_TOR_RELAY_NICKNAME", "eltor-relay");
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_DATA_DIRECTORY",
        &app_data_dir.join("tor_data_relay").to_string_lossy(),
    );
    content = content.replace("$APP_ELTOR_TOR_RELAY_OR_PORT", "9001");
    content = content.replace("$APP_ELTOR_TOR_RELAY_CONTROL_PORT", "9052");
    content = content.replace("$APP_ELTOR_TOR_RELAY_SOCKS_PORT", "9150");
    content = content.replace(
        "$APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD",
        "16:872860B76453A77D60CA2BB8C1A7042072093276A3D701AD684053EC4C",
    );
    content = content.replace("$APP_ELTOR_TOR_RELAY_ADDRESS", "127.0.0.1");
    content = content.replace("$APP_ELTOR_TOR_RELAY_CONTACT", "eltor@example.com");
    content = content.replace("$APP_ELTOR_TOR_RELAY_SANDBOX", "1");
    content = content.replace("$APP_ELTOR_TOR_EXIT_RELAY", "0");
    content = content.replace("$APP_ELTOR_TOR_RELAY_PAYMENT_RATE_MSATS", "1000");
    content = content.replace("$APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL", "300");
    content = content.replace("$APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL_ROUNDS", "10");
    content = content.replace("$APP_ELTOR_TOR_RELAY_PAYMENT_CIRCUIT_MAX_FEE", "1000");
    content = content.replace("$APP_ELTOR_LN_BOLT12", "");
    content = content.replace(
        "$APP_ELTOR_LN_CONFIG",
        "type=phoenixd url=http://localhost:9740 password=password default=true",
    );
    content = content.replace("$APP_ELTOR_TOR_RELAY_ADDITIONAL_DIR_AUTHORITY", "");

    Ok(content)
}

/// Get the torrc template content
fn get_torrc_template() -> Result<String, String> {
    // Try to read from bundled resources first
    let bin_dir = get_bin_dir()?;
    let template_path = bin_dir.join("torrc.template");

    if template_path.exists() {
        fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read torrc template: {}", e))
    } else {
        // Fallback to embedded template
        Ok(include_str!("../../../backend/bin/torrc.template").to_string())
    }
}

/// Get the torrc.relay template content
fn get_torrc_relay_template() -> Result<String, String> {
    // Try to read from bundled resources first
    let bin_dir = get_bin_dir()?;
    let template_path = bin_dir.join("torrc.relay.template");

    if template_path.exists() {
        fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read torrc.relay template: {}", e))
    } else {
        // Fallback to embedded template
        Ok(include_str!("../../../backend/bin/torrc.relay.template").to_string())
    }
}

/// Get the path for other binary files
#[allow(dead_code)]
pub fn get_binary_path(binary_name: &str) -> Result<PathBuf, String> {
    let bin_dir = get_bin_dir()?;

    #[cfg(windows)]
    let binary_name = format!("{}.exe", binary_name);

    let binary_path = bin_dir.join(binary_name);

    if !binary_path.exists() {
        return Err(format!(
            "Binary {} not found at: {:?}",
            binary_name, binary_path
        ));
    }

    Ok(binary_path)
}
