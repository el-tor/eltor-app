#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eltor_backend::eltor::EltorMode;
use eltor_backend::lightning::ListTransactionsParams;
use serde::Serialize;
use std::env;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder};
use tauri::{command, generate_context, AppHandle, Builder, Emitter, Manager, State, WindowEvent};
use tokio::sync::RwLock;

// Import backend library
use eltor_backend::{
    activate_eltord, deactivate_eltord, get_eltord_status, get_log_receiver, initialize_app_state,
    initialize_app_state_with_path_config, initialize_phoenixd, lightning, lookup_ip_location,
    ports, setup_broadcast_logger, shutdown_cleanup, torrc_parser, AppState, DebugInfo,
    IpLocationResponse, LogEntry, PathConfig, start_phoenix_with_config,
};

// Tauri-specific log entry format for frontend compatibility
#[derive(Debug, Clone, Serialize)]
struct TauriLogEntry {
    timestamp: String,
    level: String,
    message: String,
    source: String,
    mode: Option<String>,
}

impl From<LogEntry> for TauriLogEntry {
    fn from(log: LogEntry) -> Self {
        Self {
            timestamp: log.timestamp.to_rfc3339(),
            level: log.level,
            message: log.message,
            source: log.source,
            mode: log.mode,
        }
    }
}

// Simplified state wrapper for Tauri
#[derive(Clone)]
struct TauriState {
    backend_state: Arc<RwLock<AppState>>,
    log_listener_active: Arc<tokio::sync::Mutex<bool>>,
    lightning_node: Arc<tokio::sync::Mutex<Option<lightning::LightningNode>>>,
}

impl TauriState {
    fn new() -> Self {
        let backend_state = eltor_backend::create_app_state(true); // Enable embedded phoenixd for Tauri

        // Set up the broadcast logger to capture ALL logs (including from eltor library)
        if let Err(e) = setup_broadcast_logger(backend_state.clone()) {
            eprintln!("⚠️  Failed to set up broadcast logger: {}", e);
            eprintln!("   Eltor logs will go to stdout, only manual logs will stream to frontend");
        }

        Self {
            backend_state: Arc::new(RwLock::new(backend_state)),
            log_listener_active: Arc::new(tokio::sync::Mutex::new(false)),
            lightning_node: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    async fn initialize(&self) -> Result<(), String> {
        initialize_app_state(self.backend_state.clone()).await?;
        self.initialize_phoenixd().await
    }

    async fn initialize_with_app_handle(&self, app_handle: &AppHandle) -> Result<(), String> {
        let path_config = create_tauri_path_config(Some(app_handle))?;
        initialize_app_state_with_path_config(self.backend_state.clone(), path_config).await?;
        self.initialize_phoenixd().await
    }

    async fn initialize_phoenixd(&self) -> Result<(), String> {
        let app_state = self.backend_state.read().await;
        if app_state.wallet_state.use_phoenixd_embedded {
            drop(app_state);
            initialize_phoenixd(self.backend_state.clone()).await
        } else {
            println!("🔗 Using external phoenixd instance");
            Ok(())
        }
    }
}

#[command]
async fn test_log_event(app_handle: AppHandle) -> Result<String, String> {
    println!("test_log_event command called");

    let test_log = TauriLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        message: "This is a test log message from Tauri backend".to_string(),
        source: "test".to_string(),
        mode: Some("client".to_string()),
    };

    match app_handle.emit("eltord-log", &test_log) {
        Ok(_) => {
            println!("Successfully emitted test eltord-log event");
            Ok("Test log event emitted successfully".to_string())
        }
        Err(e) => {
            println!("Failed to emit test eltord-log event: {}", e);
            Err(format!("Failed to emit test event: {}", e))
        }
    }
}

#[command]
async fn activate_eltord_invoke(
    tauri_state: State<'_, TauriState>,
    app_handle: AppHandle,
    mode: String, // Accept as string and convert to EltorMode
) -> Result<String, String> {
    // Convert string mode to EltorMode
    let eltor_mode = match mode.as_str() {
        "client" => EltorMode::Client,
        "relay" => EltorMode::Relay,
        "both" => EltorMode::Both,
        _ => {
            return Err(format!(
                "Invalid mode: {}. Expected 'client', 'relay', or 'both'",
                mode
            ))
        }
    };

    let backend_state = tauri_state.backend_state.clone();

    println!(
        "🔧 Current working directory: {:?}",
        std::env::current_dir()
    );
    println!("🚀 Starting activation with mode: {:?}", eltor_mode);

    // Use the backend activate function directly
    match activate_eltord(backend_state.clone(), eltor_mode).await {
        Ok(message) => {
            println!("✅ Backend activation successful: {}", message);

            // Check if log listener is already active
            let mut listener_active = tauri_state.log_listener_active.lock().await;
            if !*listener_active {
                *listener_active = true;
                println!("📡 Starting new log listener task");

                // Set up single log listener for this activation
                let listener_flag = tauri_state.log_listener_active.clone();
                let backend_state_clone = backend_state.clone();
                tokio::spawn(async move {
                    println!("📡 Log listener task starting...");
                    let mut log_receiver = get_log_receiver(backend_state_clone).await;
                    println!("📡 Log receiver obtained, starting listen loop...");

                    let mut log_count = 0;
                    loop {
                        match log_receiver.recv().await {
                            Ok(log_entry) => {
                                log_count += 1;
                                println!(
                                    "📡 Log #{}: Received log entry from backend: {:?}",
                                    log_count, log_entry.message
                                );
                                let tauri_log: TauriLogEntry = log_entry.into();
                                match app_handle.emit("eltord-log", &tauri_log) {
                                    Ok(_) => {
                                        println!(
                                            "📡 Log #{}: Successfully emitted eltord-log event",
                                            log_count
                                        );
                                    }
                                    Err(e) => {
                                        println!(
                                            "❌ Log #{}: Failed to emit log event: {}",
                                            log_count, e
                                        );
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Log receiver error: {}", e);
                                break;
                            }
                        }
                    }

                    // Reset flag when listener stops
                    *listener_flag.lock().await = false;
                    println!(
                        "📡 Log listener task stopped after receiving {} logs",
                        log_count
                    );
                });
            } else {
                println!("📡 Log listener already active, skipping spawn");
            }

            Ok(message)
        }
        Err(e) => {
            println!("❌ Backend activation failed: {}", e);
            Err(e)
        }
    }
}

#[command]
async fn deactivate_eltord_invoke(
    tauri_state: State<'_, TauriState>,
    _app_handle: AppHandle,
    mode: String, // Add mode parameter
) -> Result<String, String> {
    println!("deactivate_eltord command called with mode: {:?}", mode);

    // Convert string mode to EltorMode
    let eltor_mode = match mode.as_str() {
        "client" => EltorMode::Client,
        "relay" => EltorMode::Relay,
        "both" => EltorMode::Both,
        _ => {
            return Err(format!(
                "Invalid mode: {}. Expected 'client', 'relay', or 'both'",
                mode
            ))
        }
    };

    let backend_state = tauri_state.backend_state.clone();

    println!("🛑 Starting deactivation with mode: {:?}", eltor_mode);

    // Use the backend deactivate function directly
    match deactivate_eltord(backend_state.clone(), eltor_mode).await {
        Ok(message) => {
            println!("✅ Backend deactivation successful: {}", message);
            Ok(message)
        }
        Err(e) => {
            println!("❌ Backend deactivation failed: {}", e);
            Err(e)
        }
    }
}

#[command]
async fn get_eltord_status_invoke(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    let backend_state = tauri_state.backend_state.clone();

    // Use backend wrapper function
    let status = get_eltord_status(backend_state.clone()).await;

    // Convert backend logs to Tauri format
    let tauri_logs: Vec<TauriLogEntry> = status
        .recent_logs
        .into_iter()
        .map(|log| log.into())
        .collect();

    // Return the new status structure with client_running and relay_running
    Ok(serde_json::json!({
        "running": status.running,
        "client_running": status.client_running,
        "relay_running": status.relay_running,
        "recent_logs": tauri_logs
    }))
}

#[command]
async fn get_node_info(tauri_state: State<'_, TauriState>) -> Result<serde_json::Value, String> {
    // Get the lightning node from TauriState
    let lightning_node_guard = tauri_state.lightning_node.lock().await;
    let lightning_node = lightning_node_guard
        .as_ref()
        .ok_or("Lightning node not initialized")?;

    // Get the balance from the lightning node
    match lightning_node.get_node_info().await {
        Ok(balance) => {
            println!(
                "✅ Send wallet balance: {} sats",
                balance.node_info.send_balance_msat / 1000
            );
            Ok(serde_json::json!(balance))
        }
        Err(e) => {
            println!("❌ Failed to get wallet balance: {}", e);
            Err(format!("Failed to get wallet balance: {}", e))
        }
    }
}

#[command]
async fn get_wallet_transactions(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    // Get the lightning node from TauriState
    let lightning_node_guard = tauri_state.lightning_node.lock().await;
    let lightning_node = lightning_node_guard
        .as_ref()
        .ok_or("Lightning node not initialized")?;

    let params = ListTransactionsParams {
        payment_hash: None, // Get all transactions
        from: 0,
        limit: 1000,
        search: None, // No search filter
    };
    match lightning_node.list_transactions(params).await {
        Ok(transactions) => Ok(serde_json::json!(transactions)),
        Err(e) => {
            println!("❌ Failed to get wallet txns: {}", e);
            Err(format!("Failed to get wallet txns: {}", e))
        }
    }
}

#[command]
async fn get_offer(tauri_state: State<'_, TauriState>) -> Result<serde_json::Value, String> {
    // Get the lightning node from TauriState
    let lightning_node_guard = tauri_state.lightning_node.lock().await;
    let lightning_node = lightning_node_guard
        .as_ref()
        .ok_or("Lightning node not initialized")?;

    // Get the offer from the lightning node
    match lightning_node.get_offer().await {
        Ok(offer) => {
            println!("✅ Retrieved BOLT12 offer: {}", offer.payment_request);
            Ok(serde_json::json!(offer))
        }
        Err(e) => {
            println!("❌ Failed to get BOLT12 offer: {}", e);
            Err(format!("Failed to get BOLT12 offer: {}", e))
        }
    }
}

#[command]
async fn lookup_ip_location_tauri(ip: String) -> Result<IpLocationResponse, String> {
    lookup_ip_location(&ip)
}

#[command]
async fn app_shutdown(tauri_state: State<'_, TauriState>) -> Result<String, String> {
    println!("🛑 App shutdown command called");

    let backend_state = tauri_state.backend_state.clone();

    // Perform comprehensive cleanup
    shutdown_cleanup(backend_state.clone()).await?;

    Ok("App shutdown cleanup completed".to_string())
}

#[command]
async fn list_lightning_configs(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    // Parse the torrc file to get all lightning configurations
    let path_config = create_tauri_path_config(Some(&app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);

    match torrc_parser::get_all_payment_lightning_configs(&torrc_path) {
        Ok(configs) => {
            println!(
                "✅ Retrieved {} lightning config(s) from torrc",
                configs.len()
            );

            let config_responses: Vec<serde_json::Value> = configs
                .into_iter()
                .map(|config| {
                    // Determine password type based on node type
                    let password_type = match config.node_type.as_str() {
                        "phoenixd" => "password",
                        "cln" => "rune",
                        "lnd" => "macaroon",
                        _ => "password", // fallback
                    };

                    serde_json::json!({
                        "node_type": config.node_type,
                        "url": config.url,
                        "password_type": password_type,
                        "password": config.password,
                        "is_default": config.is_default
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "configs": config_responses
            }))
        }
        Err(e) => {
            println!("❌ Failed to get lightning configs from torrc: {}", e);
            Err(format!("Failed to get lightning configs: {}", e))
        }
    }
}

#[command]
async fn delete_lightning_config(
    tauri_state: State<'_, TauriState>,
    app_handle: AppHandle,
    config: serde_json::Value,
) -> Result<String, String> {
    println!("🗑️  delete_lightning_config called with config: {}", config);

    let path_config = create_tauri_path_config(Some(&app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);

    // Extract config values
    let node_type_str = config["node_type"]
        .as_str()
        .ok_or("Missing node_type in config")?;

    let url = config["url"].as_str().map(|s| s.to_string());

    // Parse node type
    let node_type = match node_type_str {
        "phoenixd" => torrc_parser::NodeType::Phoenixd,
        "cln" => torrc_parser::NodeType::Cln,
        "lnd" => torrc_parser::NodeType::Lnd,
        _ => return Err(format!("Unsupported node type: {}", node_type_str)),
    };

    // Use backend torrc parser to delete the config
    match torrc_parser::modify_payment_lightning_config(
        &torrc_path,
        torrc_parser::Operation::Delete,
        node_type,
        url.clone(),
        None,
        false,
    ) {
        Ok(_) => {
            let message = match url {
                Some(url) => format!(
                    "Successfully deleted {} lightning config for {}",
                    node_type_str, url
                ),
                None => format!("Successfully deleted {} lightning config", node_type_str),
            };
            println!("✅ {}", message);

            // After deletion, try to reinitialize the lightning node in case there's a new default
            if let Err(e) = reinitialize_lightning_node(&tauri_state, &app_handle).await {
                println!(
                    "⚠️  Failed to reinitialize lightning node after deletion: {}",
                    e
                );
                println!("   This is expected if no configs remain.");
            } else {
                println!("🔄 Lightning node reinitialized after config deletion");
            }

            Ok(message)
        }
        Err(e) => {
            println!("❌ Failed to delete lightning config: {}", e);
            Err(format!("Failed to delete lightning config: {}", e))
        }
    }
}

#[command]
async fn upsert_lightning_config(
    tauri_state: State<'_, TauriState>,
    app_handle: AppHandle,
    config: serde_json::Value,
) -> Result<String, String> {
    println!("💾 upsert_lightning_config called with config: {}", config);

    let path_config = create_tauri_path_config(Some(&app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);

    // Extract config values
    let node_type_str = config["node_type"]
        .as_str()
        .ok_or("Missing node_type in config")?;
    let url = config["url"].as_str().ok_or("Missing url in config")?;
    let password = config["password"]
        .as_str()
        .ok_or("Missing password in config")?;
    let set_as_default = config["set_as_default"].as_bool().unwrap_or(false);

    // Parse node type
    let node_type = match node_type_str {
        "phoenixd" => torrc_parser::NodeType::Phoenixd,
        "cln" => torrc_parser::NodeType::Cln,
        "lnd" => torrc_parser::NodeType::Lnd,
        _ => return Err(format!("Unsupported node type: {}", node_type_str)),
    };

    // Use backend torrc parser to upsert the config
    match torrc_parser::modify_payment_lightning_config(
        &torrc_path,
        torrc_parser::Operation::Upsert,
        node_type,
        Some(url.to_string()),
        Some(password.to_string()),
        set_as_default,
    ) {
        Ok(_) => {
            let message = format!(
                "Successfully upserted {} lightning config for {}",
                node_type_str, url
            );
            println!("✅ {}", message);

            // If this config is being set as default, reinitialize the lightning node
            if set_as_default {
                if let Err(e) = reinitialize_lightning_node(&tauri_state, &app_handle).await {
                    println!("⚠️  Failed to reinitialize lightning node: {}", e);
                } else {
                    println!("🔄 Lightning node reinitialized with new default config");
                }
            }

            Ok(message)
        }
        Err(e) => {
            println!("❌ Failed to upsert lightning config: {}", e);
            Err(format!("Failed to upsert lightning config: {}", e))
        }
    }
}

/// Helper function to reinitialize the lightning node when configs change
async fn reinitialize_lightning_node(
    tauri_state: &TauriState,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let path_config = create_tauri_path_config(Some(app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);

    match lightning::LightningNode::from_torrc(&torrc_path) {
        Ok(node) => {
            println!(
                "✅ Lightning node reinitialized from torrc ({})",
                node.node_type()
            );

            // Store the new lightning node in TauriState
            let mut lightning_node_guard = tauri_state.lightning_node.lock().await;
            *lightning_node_guard = Some(node);

            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to reinitialize Lightning node from torrc: {}", e);

            // Clear the lightning node on error
            let mut lightning_node_guard = tauri_state.lightning_node.lock().await;
            *lightning_node_guard = None;

            Err(e)
        }
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let activate_i = MenuItem::with_id(app, "activate", "Activate", true, None::<&str>)?;
    let deactivate_i = MenuItem::with_id(app, "deactivate", "Deactivate", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_i, &hide_i, &activate_i, &deactivate_i, &quit_i],
    )?;

    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    println!("🛑 App quit requested - performing cleanup...");

                    match app_shutdown(tauri_state).await {
                        Ok(msg) => {
                            println!("✅ {}", msg);
                        }
                        Err(e) => {
                            println!("⚠️  Shutdown warning: {}", e);
                        }
                    }

                    // Exit after cleanup
                    app_handle.exit(0);
                });
            }
            "hide" => {
                let windows = app.webview_windows();
                windows
                    .values()
                    .next()
                    .expect("no window")
                    .hide()
                    .expect("can't hide");
            }
            "show" => {
                let windows = app.webview_windows();
                windows
                    .values()
                    .next()
                    .expect("no window")
                    .show()
                    .expect("can't show");
            }
            "activate" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    let backend_state = tauri_state.backend_state.clone();
                    // Use Client mode as default for tray activation
                    match activate_eltord(backend_state.clone(), EltorMode::Client).await {
                        Ok(msg) => {
                            println!("✅ {}", msg);
                            let _ = app_handle.emit("eltord-activated", &msg);
                        }
                        Err(err) => {
                            println!("❌ {}", err);
                            let _ = app_handle.emit("eltord-error", &err);
                        }
                    }
                });
            }
            "deactivate" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    let backend_state = tauri_state.backend_state.clone();
                    // Use Client mode as default for tray deactivation
                    match deactivate_eltord(backend_state.clone(), EltorMode::Client).await {
                        Ok(msg) => {
                            println!("✅ {}", msg);
                            let _ = app_handle.emit("eltord-deactivated", &msg);
                        }
                        Err(err) => {
                            println!("❌ {}", err);
                            let _ = app_handle.emit("eltord-error", &err);
                        }
                    }
                });
            }
            _ => {}
        })
        .build(app)?;

    println!("✅ Tray icon setup completed successfully");
    Ok(())
}

/// Create a PathConfig that's aware of Tauri's resource directory structure
/// Always uses app data directory to match production behavior
fn create_tauri_path_config(app_handle: Option<&AppHandle>) -> Result<PathConfig, String> {
    // Always use app data directory for data files (production-like behavior)
    let app_data_dir = dirs::data_dir()
        .ok_or("Failed to get app data directory")?
        .join("eltor");

    // Ensure app data directory exists with fallback to temp directory
    if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
        println!("⚠️ Warning: Could not create app data directory {:?}: {}", app_data_dir, e);
        println!("   This might be due to running from a read-only DMG. Trying temp directory fallback...");
        
        // Fallback to temporary directory
        let temp_dir = std::env::temp_dir().join("eltor");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp data directory: {}", e))?;
        println!("✅ Using temporary directory for DMG compatibility: {:?}", temp_dir);
        
        // Use temp directory for both data and app_data
        let bin_dir = if let Some(app_handle) = app_handle {
            match app_handle.path().resource_dir() {
                Ok(resource_dir) => resource_dir,
                Err(_) => {
                    // Fallback to a safe default when resource directory is not accessible
                    match std::env::current_dir() {
                        Ok(current_dir) => current_dir.join("../../backend/bin"),
                        Err(_) => std::env::temp_dir().join("eltor-bin"), // Final fallback
                    }
                }
            }
        } else {
            match std::env::current_dir() {
                Ok(current_dir) => current_dir.join("../../backend/bin"),
                Err(_) => std::env::temp_dir().join("eltor-bin"), // Final fallback
            }
        };
        
        return Ok(PathConfig {
            bin_dir,
            data_dir: temp_dir.clone(),
            app_data_dir: Some(temp_dir),
        });
    }

    if let Some(app_handle) = app_handle {
        // Try to use Tauri's resource directory for bundled resources (bins, templates)
        match app_handle.path().resource_dir() {
            Ok(resource_dir) => {
                // Check if we actually have bundled resources (production build)
                let ip_db_path = resource_dir.join("IP2LOCATION-LITE-DB3.BIN");
                let phoenixd_path = resource_dir.join("phoenixd");

                if ip_db_path.exists() || phoenixd_path.exists() {
                    println!(
                        "✅ Using Tauri resource directory for binaries: {:?}",
                        resource_dir
                    );
                    println!(
                        "✅ Using app data directory for config files: {:?}",
                        app_data_dir
                    );

                    // Production: use resource directory for binaries
                    return Ok(PathConfig {
                        bin_dir: resource_dir,
                        data_dir: app_data_dir.clone(),
                        app_data_dir: Some(app_data_dir),
                    });
                } else {
                    println!(
                        "⚠️  Tauri resource directory exists but no bundled resources found: {:?}",
                        resource_dir
                    );
                    println!("   This is expected in development mode, falling back to development bin path...");
                }
            }
            Err(e) => {
                println!("⚠️  Failed to get Tauri resource directory: {}", e);
                println!("   Falling back to development bin path...");
            }
        }
    }

    // Development fallback: Use development bin directory for binaries/templates
    println!("🔧 Using development mode with production-like app data paths");
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let bin_dir_path = current_dir.join("../../backend/bin");
    
    // Try to canonicalize, but if it fails, just use the app data directory for everything
    let bin_dir = match bin_dir_path.canonicalize() {
        Ok(canonical_path) => {
            // Verify development bin directory exists and has required files
            let phoenixd_path = canonical_path.join("phoenixd");
            let ip_db_path = canonical_path.join("IP2LOCATION-LITE-DB3.BIN");

            if phoenixd_path.exists() || ip_db_path.exists() {
                println!("✅ Using development bin directory: {:?}", canonical_path);
                canonical_path
            } else {
                println!("⚠️  Development bin directory found but missing required files, using app data directory for templates");
                app_data_dir.clone()
            }
        }
        Err(_) => {
            println!("⚠️  Development bin directory not found (expected in bundled builds), using app data directory for templates");
            app_data_dir.clone()
        }
    };

    println!(
        "✅ Using app data directory for config files: {:?}",
        app_data_dir
    );

    Ok(PathConfig {
        bin_dir,
        data_dir: app_data_dir.clone(),
        app_data_dir: Some(app_data_dir),
    })
}

#[command]
async fn get_debug_info(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let path_config = create_tauri_path_config(Some(&app_handle))?;
    let debug_info = DebugInfo::with_path_config(path_config)?;

    Ok(serde_json::to_value(&debug_info)
        .map_err(|e| format!("Failed to serialize debug info: {}", e))?)
}

#[command]
async fn start_phoenix_daemon(
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    println!("🔥 start_phoenix_daemon called");

    let path_config = create_tauri_path_config(Some(&app_handle))?;

    // Use the new start_phoenix_with_config function
    match start_phoenix_with_config(&path_config).await {
        Ok(response) => {
            println!("✅ Phoenix daemon started successfully with config: {:?}", response);
            Ok(serde_json::to_value(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?)
        }
        Err(e) => {
            println!("❌ Failed to start Phoenix daemon: {}", e);
            Err(format!("Failed to start Phoenix daemon: {}", e))
        }
    }
}

#[command]
async fn stop_phoenix_daemon(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    println!("🛑 stop_phoenix_daemon called");

    // Stop phoenixd using the existing backend function
    let backend_state = tauri_state.backend_state.read().await;
    let app_state = backend_state.clone();
    drop(backend_state);
    
    match eltor_backend::stop_phoenixd(app_state).await {
        Ok(()) => {
            println!("✅ Phoenix daemon stopped successfully");
            
            let response = serde_json::json!({
                "success": true,
                "message": "Phoenix daemon stopped successfully",
                "pid": null // We could get PID from AppState if needed
            });
            Ok(response)
        }
        Err(e) => {
            if e.contains("No phoenixd process") || e.contains("not running") {
                println!("ℹ️  Phoenix daemon was not running");
                let response = serde_json::json!({
                    "success": true,
                    "message": "Phoenix daemon is not currently running",
                    "pid": null
                });
                Ok(response)
            } else {
                println!("❌ Failed to stop Phoenix daemon: {}", e);
                Err(format!("Failed to stop Phoenix daemon: {}", e))
            }
        }
    }
}

fn main() {
    // Load environment variables from root .env file
    dotenv::from_path("../../.env").ok();
    // Print environment variables for debugging
    println!("🔧 Tauri Environment variables:");
    for (key, value) in env::vars() {
        if key.starts_with("APP_") || key.starts_with("BACKEND_") || key.starts_with("PHOENIXD_") {
            println!("   {}: {}", key, value);
        }
    }

    // Initialize torrc files before starting the app (using development fallback)
    // This ensures torrc files exist even before we have an app handle
    let path_config = create_tauri_path_config(None);
    match path_config {
        Ok(config) => {
            if let Err(e) = config.ensure_torrc_files() {
                eprintln!("⚠️  Failed to initialize torrc files: {}", e);
                eprintln!("   Continuing with startup...");
            } else {
                println!("✅ Pre-startup torrc files ensured with app data directory");
            }
        }
        Err(e) => {
            eprintln!("⚠️  Failed to get path configuration: {}", e);
            eprintln!("   Continuing with startup...");
        }
    }

    Builder::default()
        .setup(|app| {
            setup_tray(app.handle())?;

            // Re-initialize with proper app context for production builds
            let app_config = create_tauri_path_config(Some(app.handle()));
            match app_config {
                Ok(config) => {
                    if let Err(e) = config.ensure_torrc_files() {
                        eprintln!("⚠️  Failed to re-initialize torrc files with app context: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to get app path configuration: {}", e);
                }
            }

            // Initialize the Tauri state
            let tauri_state = TauriState::new();

            // Initialize phoenixd asynchronously after the runtime is available
            let app_handle = app.handle().clone();
            let state_for_init = tauri_state.clone();

            tauri::async_runtime::spawn(async move {
                // Initialize the state with EltorManager first, using proper Tauri PathConfig
                if let Err(e) = state_for_init.initialize_with_app_handle(&app_handle).await {
                    eprintln!("❌ Failed to initialize Tauri state: {}", e);
                    return;
                }

                // Clean up any processes using our ports
                println!("🧹 Starting port cleanup...");
                if let Err(e) = ports::cleanup_ports_with_torrc("torrc").await {
                    eprintln!("⚠️  Port cleanup failed: {}", e);
                    eprintln!("   Continuing with startup...");
                }

                let use_phoenixd_embedded = env::var("APP_ELTOR_USE_PHOENIXD_EMBEDDED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse::<bool>()
                    .unwrap_or(false);

                if use_phoenixd_embedded {
                    match state_for_init.initialize_phoenixd().await {
                        Ok(_) => {
                            println!("✅ Phoenixd initialization completed successfully");
                            let _ = app_handle.emit("phoenixd-ready", "Phoenixd wallet ready");
                        }
                        Err(e) => {
                            println!("❌ Failed to initialize phoenixd: {}", e);
                            let _ = app_handle.emit(
                                "phoenixd-error",
                                format!("Phoenixd initialization failed: {}", e),
                            );
                        }
                    }
                }

                // Initialize lightning node
                let torrc_path = match create_tauri_path_config(Some(&app_handle)) {
                    Ok(path_config) => {
                        if let Err(e) = path_config.ensure_torrc_files() {
                            println!("❌ Failed to ensure torrc files: {}", e);
                            return;
                        }
                        path_config.get_torrc_path(None)
                    },
                    Err(e) => {
                        println!("❌ Failed to get path config: {}", e);
                        return;
                    }
                };
                println!("🔎 Lightning torrc path: {:?}", torrc_path);
                match lightning::LightningNode::from_torrc(torrc_path) {
                    Ok(node) => {
                        println!(
                            "✅ Lightning node connected from torrc ({})",
                            node.node_type()
                        );
                        // Store the lightning node in TauriState
                        let mut lightning_node_guard = state_for_init.lightning_node.lock().await;
                        *lightning_node_guard = Some(node);
                        drop(lightning_node_guard); // Release the lock
                        let _ = app_handle.emit("lightning-ready", "Lightning node ready");
                    }
                    Err(e) => {
                        println!("⚠️  Failed to initialize Lightning node from torrc: {}", e);
                        let _ = app_handle.emit(
                            "lightning-error",
                            format!("Lightning initialization failed: {}", e),
                        );
                    }
                }

                // Initialize IP database for Tauri
                let ip_db_path = match create_tauri_path_config(Some(&app_handle)) {
                    Ok(path_config) => path_config.get_executable_path("IP2LOCATION-LITE-DB3.BIN"),
                    Err(e) => {
                        eprintln!("⚠️  Failed to get path config: {}", e);
                        return;
                    }
                };
                if ip_db_path.exists() {
                    println!("✅ IP database file found at: {:?}", ip_db_path);
                } else {
                    eprintln!("⚠️  IP database not found. Download IP2LOCATION-LITE-DB3.BIN from https://download.ip2location.com/lite/");
                }
            });

            // Store the state for Tauri commands
            app.manage(tauri_state);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            activate_eltord_invoke,
            deactivate_eltord_invoke,
            get_eltord_status_invoke,
            test_log_event,
            get_node_info,
            get_wallet_transactions,
            get_offer,
            lookup_ip_location_tauri,
            app_shutdown,
            list_lightning_configs,
            delete_lightning_config,
            upsert_lightning_config,
            get_debug_info,
            start_phoenix_daemon,
            stop_phoenix_daemon
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
