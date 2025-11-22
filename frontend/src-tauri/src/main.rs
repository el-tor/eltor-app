#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eltor_backend::eltor::EltorMode;
use eltor_backend::lightning::ListTransactionsParams;
use serde::{Deserialize, Serialize};
use std::{env};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{command, generate_context, AppHandle, Builder, Emitter, Manager, State, WindowEvent, Theme};
use tokio::sync::RwLock;
use log::info;

// Import backend library
use eltor_backend::{
    activate_eltord, deactivate_eltord, get_eltord_logs,
    stream_eltord_logs_internal, init_ip_database,
    initialize_app_state_with_path_config, initialize_phoenixd, lightning,
    lookup_ip_location, ports, shutdown_cleanup, torrc_parser, AppState,
    DebugInfo, IpLocationResponse, LogEntry, PathConfig, start_phoenix_with_config,
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
    // Note: lightning_node is now managed in backend_state.lightning_node
    // Removed redundant TauriState.lightning_node to use the cached version
}

impl TauriState {
    fn new() -> Self {
        // Create a default PathConfig (will be updated later with proper resource dir)
        let path_config = PathConfig::new().unwrap_or_else(|_| {
            PathConfig::with_overrides(
                Some(std::env::current_dir().unwrap().join("bin")),
                Some(std::env::current_dir().unwrap().join("bin/data")),
            )
            .unwrap()
        });
        
        let backend_state = eltor_backend::create_app_state(true, path_config); // Enable embedded phoenixd for Tauri

        // Set up the broadcast logger to capture ALL logs (including from eltor library)
        // if let Err(e) = setup_broadcast_logger(backend_state.clone()) {
        //     info!("⚠️  Failed to set up broadcast logger: {}", e);
        //     info!("   Eltor logs will go to stdout, only manual logs will stream to frontend");
        // }

        Self {
            backend_state: Arc::new(RwLock::new(backend_state)),
            log_listener_active: Arc::new(tokio::sync::Mutex::new(false)),
        }
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
            info!("🔗 Using external phoenixd instance");
            Ok(())
        }
    }
}

#[command]
async fn test_log_event(app_handle: AppHandle) -> Result<String, String> {
    info!("test_log_event command called");

    let test_log = TauriLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        message: "This is a test log message from Tauri backend".to_string(),
        source: "test".to_string(),
        mode: Some("client".to_string()),
    };

    match app_handle.emit("eltord-log", &test_log) {
        Ok(_) => {
            info!("Successfully emitted test eltord-log event");
            Ok("Test log event emitted successfully".to_string())
        }
        Err(e) => {
            info!("Failed to emit test eltord-log event: {}", e);
            Err(format!("Failed to emit test event: {}", e))
        }
    }
}

#[command]
fn activate_eltord_invoke(mode: String, enable_logging: Option<bool>) -> Result<String, String>  {
    info!(
        "🔧 Current working directory: {:?}",
        std::env::current_dir()
    );
    // info!("🚀 Starting activation with mode: {:?}", mode);
    let enable_logging = enable_logging.unwrap_or(false);
    eltor_backend::eltor::activate_eltord_process(mode, enable_logging);
    Ok("Activation started".to_string())
}

#[command]
async fn deactivate_eltord_invoke(
    mode: String,
) -> Result<String, String> {
    info!("🛑 deactivate_eltord_invoke command called with mode: {:?}", mode);
    
    // Use the async PID file-based deactivation with graceful shutdown
    eltor_backend::eltor::deactivate_eltord_process(mode).await
}

#[command]
async fn get_eltord_status_invoke(
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    // Get the path config for this Tauri instance
    let path_config = create_tauri_path_config(Some(&app_handle))?;
    
    // Use the backend function that checks PID files directly
    let status = eltor_backend::eltor::get_eltord_status_from_pid_files(&path_config).await;

    // Return the status structure with client_running and relay_running
    Ok(serde_json::json!({
        "running": status.running,
        "client_running": status.client_running,
        "relay_running": status.relay_running,
        "recent_logs": [] // No logs in status response, use streaming for logs
    }))
}

#[command]
async fn get_eltord_logs_invoke(
    tauri_state: State<'_, TauriState>,
    mode: String,
) -> Result<Vec<String>, String> {
    let backend_state = tauri_state.backend_state.clone();
    get_eltord_logs(backend_state, mode).await
}

#[command]
async fn stream_eltord_logs_invoke(
    tauri_state: State<'_, TauriState>,
    app_handle: AppHandle,
    mode: String,
) -> Result<(), String> {
    let backend_state = tauri_state.backend_state.clone();
    
    // Create emit function that captures app_handle
    let emit_fn = Arc::new(move |message: String| {
        let _ = app_handle.emit("eltord-log", message);
    });
    
    stream_eltord_logs_internal(backend_state, mode, emit_fn).await
}

#[command]
async fn stop_eltord_logs_invoke(
    tauri_state: State<'_, TauriState>,
    mode: String,
) -> Result<(), String> {
    let backend_state = tauri_state.backend_state.read().await;
    
    // Cancel the appropriate log stream
    if mode == "client" {
        let mut client_cancel = backend_state.client_log_cancel.lock().unwrap();
        if let Some(token) = client_cancel.take() {
            token.cancel();
            info!("⏸️ [Tauri] Cancelled client log stream");
        }
    } else if mode == "relay" {
        let mut relay_cancel = backend_state.relay_log_cancel.lock().unwrap();
        if let Some(token) = relay_cancel.take() {
            token.cancel();
            info!("⏸️ [Tauri] Cancelled relay log stream");
        }
    }
    
    Ok(())
}

#[command]
async fn get_node_info(tauri_state: State<'_, TauriState>) -> Result<serde_json::Value, String> {
    // Get the lightning node from backend AppState (cached)
    let backend_state = tauri_state.backend_state.read().await;
    
    // Clone the lightning node to avoid holding the lock across await
    let lightning_node = {
        let lightning_node_guard = backend_state.lightning_node.lock().unwrap();
        lightning_node_guard.clone()
    }; // Guard is dropped here
    
    if let Some(lightning_node) = lightning_node {
        // Get the balance from the lightning node
        match lightning_node.get_node_info().await {
            Ok(balance) => {
                info!(
                    "✅ Send wallet balance: {} sats",
                    balance.node_info.send_balance_msat / 1000
                );
                Ok(serde_json::json!(balance))
            }
            Err(e) => {
                info!("❌ Failed to get wallet balance: {}", e);
                Err(format!("Failed to get wallet balance: {}", e))
            }
        }
    } else {
        Err("Lightning node not initialized".to_string())
    }
}

#[command]
async fn get_wallet_transactions(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    // Get the lightning node from backend AppState (cached)
    let backend_state = tauri_state.backend_state.read().await;
    
    // Clone the lightning node to avoid holding the lock across await
    let lightning_node = {
        let lightning_node_guard = backend_state.lightning_node.lock().unwrap();
        lightning_node_guard.clone()
    }; // Guard is dropped here
    
    if let Some(lightning_node) = lightning_node {
        let params = ListTransactionsParams {
            payment_hash: None, // Get all transactions
            from: 0,
            limit: 1000,
            search: None, // No search filter
        };
        match lightning_node.list_transactions(params).await {
            Ok(transactions) => Ok(serde_json::json!(transactions)),
            Err(e) => {
                info!("❌ Failed to get wallet txns: {}", e);
                Err(format!("Failed to get wallet txns: {}", e))
            }
        }
    } else {
        Err("Lightning node not initialized".to_string())
    }
}

#[command]
async fn get_offer(tauri_state: State<'_, TauriState>) -> Result<serde_json::Value, String> {
    // Get the lightning node from backend AppState (cached)
    let backend_state = tauri_state.backend_state.read().await;
    
    // Clone the lightning node to avoid holding the lock across await
    let lightning_node = {
        let lightning_node_guard = backend_state.lightning_node.lock().unwrap();
        lightning_node_guard.clone()
    }; // Guard is dropped here
    
    if let Some(lightning_node) = lightning_node {
        // Get the offer from the lightning node
        match lightning_node.get_offer().await {
            Ok(offer) => {
                info!("✅ Retrieved BOLT12 offer: {}", offer.payment_request);
                Ok(serde_json::json!(offer))
            }
            Err(e) => {
                info!("❌ Failed to get BOLT12 offer: {}", e);
                Err(format!("Failed to get BOLT12 offer: {}", e))
            }
        }
    } else {
        Err("Lightning node not initialized".to_string())
    }
}

#[command]
async fn lookup_ip_location_tauri(ip: String) -> Result<IpLocationResponse, String> {
    lookup_ip_location(&ip)
}

#[command]
async fn app_shutdown(tauri_state: State<'_, TauriState>) -> Result<String, String> {
    info!("🛑 App shutdown command called");

    let backend_state = tauri_state.backend_state.clone();

    // Perform comprehensive cleanup
    shutdown_cleanup(backend_state.clone()).await?;

    Ok("App shutdown cleanup completed".to_string())
}

// Helper function for direct cleanup (used by signal handlers)
async fn perform_cleanup(tauri_state: &TauriState) -> Result<String, String> {
    info!("🛑 perform_cleanup() called - starting cleanup process...");

    let backend_state = tauri_state.backend_state.clone();

    // Perform comprehensive cleanup
    shutdown_cleanup(backend_state.clone()).await?;

    info!("✅ perform_cleanup() completed successfully");
    Ok("Cleanup completed".to_string())
}

#[command]
async fn list_lightning_configs(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    // Parse the torrc file to get all lightning configurations
    let path_config = create_tauri_path_config(Some(&app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_path = path_config.get_torrc_path(None);

    match torrc_parser::get_all_payment_lightning_configs(&torrc_path).await {
        Ok(configs) => {
            info!(
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
            info!("❌ Failed to get lightning configs from torrc: {}", e);
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
    info!("🗑️  delete_lightning_config called with config: {}", config);

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
    ).await {
        Ok(_) => {
            let message = match url {
                Some(url) => format!(
                    "Successfully deleted {} lightning config for {}",
                    node_type_str, url
                ),
                None => format!("Successfully deleted {} lightning config", node_type_str),
            };
            info!("✅ {}", message);

            // After deletion, try to reinitialize the lightning node in case there's a new default
            if let Err(e) = reinitialize_lightning_node(&tauri_state, &app_handle).await {
                info!(
                    "⚠️  Failed to reinitialize lightning node after deletion: {}",
                    e
                );
                info!("   This is expected if no configs remain.");
            } else {
                info!("🔄 Lightning node reinitialized after config deletion");
            }

            Ok(message)
        }
        Err(e) => {
            info!("❌ Failed to delete lightning config: {}", e);
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
    info!("💾 upsert_lightning_config called with config: {}", config);

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
    ).await {
        Ok(_) => {
            let message = format!(
                "Successfully upserted {} lightning config for {}",
                node_type_str, url
            );
            info!("✅ {}", message);

            // If this config is being set as default, reinitialize the lightning node
            if set_as_default {
                if let Err(e) = reinitialize_lightning_node(&tauri_state, &app_handle).await {
                    info!("⚠️  Failed to reinitialize lightning node: {}", e);
                } else {
                    info!("🔄 Lightning node reinitialized with new default config");
                }
            }

            Ok(message)
        }
        Err(e) => {
            info!("❌ Failed to upsert lightning config: {}", e);
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

    match lightning::LightningNode::from_torrc(&torrc_path).await {
        Ok(node) => {
            info!(
                "✅ Lightning node reinitialized from torrc ({})",
                node.node_type()
            );

            // Store the new lightning node in backend AppState (cached)
            let backend_state = tauri_state.backend_state.read().await;
            backend_state.set_lightning_node(node);

            Ok(())
        }
        Err(e) => {
            info!("❌ Failed to reinitialize Lightning node from torrc: {}", e);

            // Clear the lightning node on error
            let backend_state = tauri_state.backend_state.read().await;
            let mut lightning_node_guard = backend_state.lightning_node.lock().unwrap();
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

    let app_clone = app.clone();
    
    // Try to load tray icon from resources, with fallback
    // On Linux, use white icon for better visibility on dark panels
    #[cfg(target_os = "linux")]
    let icon_name = "tray-icon-white.png";
    #[cfg(not(target_os = "linux"))]
    let icon_name = "tray-icon.png";
    
    let tray_icon = match app.path().resource_dir() {
        Ok(resource_dir) => {
            // Icons are in Resources/icons/ directory
            let icon_path = resource_dir.join("icons").join(icon_name);
            info!("🖼️  Attempting to load tray icon from: {:?}", icon_path);
            
            match tauri::image::Image::from_path(&icon_path) {
                Ok(img) => {
                    info!("✅ Tray icon loaded successfully");
                    img
                }
                Err(e) => {
                    info!("⚠️  Failed to load tray icon from resources: {}", e);
                    info!("   Trying local path fallback...");
                    
                    // Fallback to local path (development mode)
                    let fallback_path = format!("icons/{}", icon_name);
                    match tauri::image::Image::from_path(&fallback_path) {
                        Ok(img) => {
                            info!("✅ Tray icon loaded from local path");
                            img
                        }
                        Err(e) => {
                            info!("⚠️  Failed to load tray icon from local path: {}", e);
                            info!("   Trying default tray-icon.png as final fallback...");
                            // Final fallback to standard icon
                            match tauri::image::Image::from_path("icons/tray-icon.png") {
                                Ok(img) => {
                                    info!("✅ Tray icon loaded from fallback");
                                    img
                                }
                                Err(e) => {
                                    info!("⚠️  Failed to load any tray icon: {}", e);
                                    return Err(tauri::Error::AssetNotFound(format!("Tray icon not found: {}", e)));
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            info!("⚠️  Could not get resource directory: {}", e);
            info!("   Trying local path...");
            
            let fallback_path = format!("icons/{}", icon_name);
            match tauri::image::Image::from_path(&fallback_path) {
                Ok(img) => {
                    info!("✅ Tray icon loaded from local path");
                    img
                }
                Err(e) => {
                    info!("⚠️  Failed to load tray icon: {}", e);
                    // Final fallback
                    match tauri::image::Image::from_path("icons/tray-icon.png") {
                        Ok(img) => {
                            info!("✅ Tray icon loaded from fallback");
                            img
                        }
                        Err(e) => {
                            return Err(tauri::Error::AssetNotFound(format!("Tray icon not found: {}", e)));
                        }
                    }
                }
            }
        }
    };
    
    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(tray_icon)
        .icon_as_template(true)
        .on_tray_icon_event(move |_tray, event| {
            // Handle tray icon click events to show/hide window
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app_handle = app_clone.clone();
                if let Some(window) = app_handle.webview_windows().values().next() {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    info!("🛑 App quit requested - performing cleanup...");

                    match app_shutdown(tauri_state).await {
                        Ok(msg) => {
                            info!("✅ {}", msg);
                        }
                        Err(e) => {
                            info!("⚠️  Shutdown warning: {}", e);
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
                let _ = activate_eltord("client".to_string(), false);
            }
            "deactivate" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    let backend_state = tauri_state.backend_state.clone();
                    // Use Client mode as default for tray deactivation
                    match deactivate_eltord(backend_state.clone(), EltorMode::Client).await {
                        Ok(msg) => {
                            info!("✅ {}", msg);
                            let _ = app_handle.emit("eltord-deactivated", &msg);
                        }
                        Err(err) => {
                            info!("❌ {}", err);
                            let _ = app_handle.emit("eltord-error", &err);
                        }
                    }
                });
            }
            _ => {}
        })
        .build(app)?;

    info!("✅ Tray icon setup completed successfully");
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
        info!("⚠️ Warning: Could not create app data directory {:?}: {}", app_data_dir, e);
        info!("   This might be due to running from a read-only DMG. Trying temp directory fallback...");
        
        // Fallback to temporary directory
        let temp_dir = std::env::temp_dir().join("eltor");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp data directory: {}", e))?;
        info!("✅ Using temporary directory for DMG compatibility: {:?}", temp_dir);
        
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
                // First check for Tauri's _up_ directory structure (when resources use relative paths)
                let tauri_bin_dir = resource_dir.join("_up_").join("_up_").join("backend").join("bin");
                
                if tauri_bin_dir.exists() {
                    let phoenixd_path = tauri_bin_dir.join("phoenixd");
                    let ip_db_path = tauri_bin_dir.join("IP2LOCATION-LITE-DB3.BIN");
                    
                    if phoenixd_path.exists() || ip_db_path.exists() {
                        info!(
                            "✅ Using Tauri _up_ structure for binaries: {:?}",
                            tauri_bin_dir
                        );
                        info!(
                            "✅ Using app data directory for config files: {:?}",
                            app_data_dir
                        );

                        // Production: use _up_ directory for binaries
                        return Ok(PathConfig {
                            bin_dir: tauri_bin_dir,
                            data_dir: app_data_dir.clone(),
                            app_data_dir: Some(app_data_dir),
                        });
                    }
                }
                
                // Fallback: check if files are directly in Resources (alternative bundle structure)
                let ip_db_path = resource_dir.join("IP2LOCATION-LITE-DB3.BIN");
                let phoenixd_path = resource_dir.join("phoenixd");

                if ip_db_path.exists() || phoenixd_path.exists() {
                    info!(
                        "✅ Using Tauri resource directory for binaries: {:?}",
                        resource_dir
                    );
                    info!(
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
                    info!(
                        "⚠️  Tauri resource directory exists but no bundled resources found: {:?}",
                        resource_dir
                    );
                    info!("   This is expected in development mode, falling back to development bin path...");
                }
            }
            Err(e) => {
                info!("⚠️  Failed to get Tauri resource directory: {}", e);
                info!("   Falling back to development bin path...");
            }
        }
    }

    // Development fallback: Use development bin directory for binaries/templates
    info!("🔧 Using development mode with production-like app data paths");
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
                info!("✅ Using development bin directory: {:?}", canonical_path);
                canonical_path
            } else {
                info!("⚠️  Development bin directory found but missing required files, using app data directory for templates");
                app_data_dir.clone()
            }
        }
        Err(_) => {
            info!("⚠️  Development bin directory not found (expected in bundled builds), using app data directory for templates");
            app_data_dir.clone()
        }
    };

    info!(
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
    let debug_info = DebugInfo::with_path_config(path_config).await?;

    serde_json::to_value(&debug_info)
        .map_err(|e| format!("Failed to serialize debug info: {}", e))
}

#[command]
async fn start_phoenix_daemon(
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    info!("🔥 start_phoenix_daemon called");

    let path_config = create_tauri_path_config(Some(&app_handle))?;

    // Use the new start_phoenix_with_config function
    match start_phoenix_with_config(&path_config).await {
        Ok(response) => {
            info!("✅ Phoenix daemon started successfully with config: {:?}", response);
            Ok(serde_json::to_value(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?)
        }
        Err(e) => {
            info!("❌ Failed to start Phoenix daemon: {}", e);
            Err(format!("Failed to start Phoenix daemon: {}", e))
        }
    }
}

#[command]
async fn stop_phoenix_daemon(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    info!("🛑 stop_phoenix_daemon called");

    // Stop phoenixd using the existing backend function
    let backend_state = tauri_state.backend_state.read().await;
    let app_state = backend_state.clone();
    drop(backend_state);
    
    match eltor_backend::stop_phoenixd(app_state).await {
        Ok(()) => {
            info!("✅ Phoenix daemon stopped successfully");
            
            let response = serde_json::json!({
                "success": true,
                "message": "Phoenix daemon stopped successfully",
                "pid": null // We could get PID from AppState if needed
            });
            Ok(response)
        }
        Err(e) => {
            if e.contains("No phoenixd process") || e.contains("not running") {
                info!("ℹ️  Phoenix daemon was not running");
                let response = serde_json::json!({
                    "success": true,
                    "message": "Phoenix daemon is not currently running",
                    "pid": null
                });
                Ok(response)
            } else {
                info!("❌ Failed to stop Phoenix daemon: {}", e);
                Err(format!("Failed to stop Phoenix daemon: {}", e))
            }
        }
    }
}

#[command]
async fn update_relay_payment_rate(
    app_handle: AppHandle,
    #[allow(non_snake_case)]
    rateSatsPerMin: f64,
) -> Result<serde_json::Value, String> {
    info!("💰 update_relay_payment_rate called with rate: {} sats/min", rateSatsPerMin);

    let path_config = create_tauri_path_config(Some(&app_handle))?;
    path_config.ensure_torrc_files()?;
    let torrc_relay_path = path_config.get_torrc_relay_path();

    // Convert sats/min to msats/min (1 sat = 1000 msats)
    let rate_msats = (rateSatsPerMin * 1000.0) as u64;

    // Update PaymentRateMsats in torrc.relay
    torrc_parser::update_torrc_config_line(
        &torrc_relay_path,
        "PaymentRateMsats",
        &rate_msats.to_string(),
    )
    .await
    .map_err(|e| format!("Failed to update payment rate: {}", e))?;

    info!("✅ Payment rate updated to {} msats/min ({} sats/min)", rate_msats, rateSatsPerMin);

    Ok(serde_json::json!({
        "message": format!("Payment rate updated to {} msats/min ({} sats/min)", rate_msats, rateSatsPerMin),
        "rate_msats": rate_msats
    }))
}

#[command]
async fn detect_phoenix_config(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    info!("🔍 detect_phoenix_config called");

    // Check if Phoenix process is running in our state
    let backend_state = tauri_state.backend_state.read().await;
    let phoenixd_process = backend_state.wallet_state.phoenixd_process.lock().unwrap();
    let is_running = phoenixd_process.is_some();
    drop(phoenixd_process);
    drop(backend_state);

    // Try to get existing Phoenix config from ~/.phoenix/phoenix.conf
    let home_dir = dirs::home_dir().ok_or("Could not get home directory")?;
    let phoenix_conf_path = home_dir.join(".phoenix").join("phoenix.conf");
    
    if phoenix_conf_path.exists() {
        // Try to read the password from config
        match std::fs::read_to_string(&phoenix_conf_path) {
            Ok(conf_content) => {
                let mut password = String::new();
                for line in conf_content.lines() {
                    let line = line.trim();
                    if line.starts_with("http-password=") {
                        if let Some(pwd) = line.strip_prefix("http-password=") {
                            password = pwd.to_string();
                            break;
                        }
                    }
                }
                
                if !password.is_empty() {
                    info!("✅ Found existing Phoenix configuration (running: {})", is_running);
                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!("Existing Phoenix configuration detected (running: {})", is_running),
                        "downloaded": false,
                        "pid": null,
                        "url": "http://127.0.0.1:9740",
                        "password": password,
                        "is_running": is_running,
                    }))
                } else {
                    Err("http-password not found in phoenix.conf".to_string())
                }
            }
            Err(e) => {
                Err(format!("Failed to read phoenix.conf: {}", e))
            }
        }
    } else if is_running {
        // Config doesn't exist but process is running
        info!("⚠️  Phoenix is running but configuration not yet available");
        Ok(serde_json::json!({
            "success": true,
            "message": "Phoenix is running but configuration not yet available",
            "downloaded": false,
            "pid": null,
            "url": "http://127.0.0.1:9740",
            "password": null,
            "is_running": true,
        }))
    } else {
        Err("Phoenix config file not found and Phoenix is not running".to_string())
    }
}

fn main() {
    // Load environment variables from root .env file
    dotenv::from_path("../../.env").ok();
    std::env::set_var("ELTOR_TAURI_MODE", "1");
    
    // Set bin directory path for backend to use
    // This needs to be done early, before any backend functions are called
    // We'll update it later in setup() once we have the app handle
    
    // Print environment variables for debugging
    info!("🔧 Tauri Environment variables:");
    for (key, value) in env::vars() {
        if key.starts_with("APP_") || key.starts_with("BACKEND_") || key.starts_with("PHOENIXD_") || key.starts_with("ELTOR_") {
            info!("   {}: {}", key, value);
        }
    }

    // Initialize torrc files before starting the app (using development fallback)
    // This ensures torrc files exist even before we have an app handle
    let path_config = create_tauri_path_config(None);
    match path_config {
        Ok(config) => {
            if let Err(e) = config.ensure_torrc_files() {
                info!("⚠️  Failed to initialize torrc files: {}", e);
                info!("   Continuing with startup...");
            } else {
                info!("✅ Pre-startup torrc files ensured with app data directory");
            }
        }
        Err(e) => {
            info!("⚠️  Failed to get path configuration: {}", e);
            info!("   Continuing with startup...");
        }
    }

    Builder::default()
        .setup(|app| {
            // Set up tray with error handling - don't fail the entire app if tray fails
            if let Err(e) = setup_tray(app.handle()) {
                info!("⚠️  Failed to set up system tray: {}", e);
                info!("   App will continue without system tray");
            } else {
                info!("✅ System tray initialized successfully");
            }

            // Configure window to match app's dark theme on macOS
            #[cfg(target_os = "macos")]
            {
                use tauri::Theme;
                // Get the first/main window
                if let Some(main_window) = app.webview_windows().values().next() {
                    // Set theme to Dark to match the app's dark background
                    if let Err(e) = main_window.set_theme(Some(Theme::Dark)) {
                        info!("⚠️  Failed to set dark theme: {}", e);
                    } else {
                        info!("✅ Window theme set to dark to match app background");
                    }
                } else {
                    info!("⚠️  No main window found for theme configuration");
                }
            }

            // Re-initialize with proper app context for production builds
            let app_config = create_tauri_path_config(Some(app.handle()));
            match app_config {
                Ok(config) => {
                    // Set the bin_dir as an environment variable for the backend to use
                    std::env::set_var("ELTOR_TAURI_BIN_DIR", &config.bin_dir);
                    info!("✅ Set ELTOR_TAURI_BIN_DIR={:?}", config.bin_dir);
                    
                    if let Err(e) = config.ensure_torrc_files() {
                        info!("⚠️  Failed to re-initialize torrc files with app context: {}", e);
                    }
                }
                Err(e) => {
                    info!("⚠️  Failed to get app path configuration: {}", e);
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
                    info!("❌ Failed to initialize Tauri state: {}", e);
                    return;
                }

                // Clean up any processes using our ports
                info!("🧹 Starting port cleanup...");
                if let Err(e) = ports::cleanup_ports_with_torrc("torrc").await {
                    info!("⚠️  Port cleanup failed: {}", e);
                    info!("   Continuing with startup...");
                }

                let use_phoenixd_embedded = env::var("APP_ELTOR_USE_PHOENIXD_EMBEDDED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse::<bool>()
                    .unwrap_or(false);

                if use_phoenixd_embedded {
                    match state_for_init.initialize_phoenixd().await {
                        Ok(_) => {
                            info!("✅ Phoenixd initialization completed successfully");
                            let _ = app_handle.emit("phoenixd-ready", "Phoenixd wallet ready");
                        }
                        Err(e) => {
                            info!("❌ Failed to initialize phoenixd: {}", e);
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
                            info!("❌ Failed to ensure torrc files: {}", e);
                            return;
                        }
                        path_config.get_torrc_path(None)
                    },
                    Err(e) => {
                        info!("❌ Failed to get path config: {}", e);
                        return;
                    }
                };
                info!("🔎 Lightning torrc path: {:?}", torrc_path);
                match lightning::LightningNode::from_torrc(torrc_path).await {
                    Ok(node) => {
                        info!(
                            "✅ Lightning node connected from torrc ({})",
                            node.node_type()
                        );
                        // Store the lightning node in backend AppState (cached)
                        let backend_state = state_for_init.backend_state.read().await;
                        backend_state.set_lightning_node(node);
                        drop(backend_state); // Release the lock
                        let _ = app_handle.emit("lightning-ready", "Lightning node ready");
                    }
                    Err(e) => {
                        info!("⚠️  Failed to initialize Lightning node from torrc: {}", e);
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
                        info!("⚠️  Failed to get path config: {}", e);
                        return;
                    }
                };
                if ip_db_path.exists() {
                    info!("✅ IP database file found at: {:?}", ip_db_path);
                    match init_ip_database(ip_db_path.clone()) {
                        Ok(()) => info!("✅ IP database initialized successfully"),
                        Err(e) => info!("❌ Failed to initialize IP database: {}", e),
                    }
                } else {
                    info!("⚠️  IP database not found. Download IP2LOCATION-LITE-DB3.BIN from https://download.ip2location.com/lite/");
                }
            });

            // Store the state for Tauri commands
            app.manage(tauri_state.clone());

            // Setup Ctrl+C signal handler for graceful shutdown when debugging
            let app_handle_for_signal = app.handle().clone();
            let state_for_signal = tauri_state.clone();
            std::thread::spawn(move || {
                tauri::async_runtime::block_on(async move {
                    match tokio::signal::ctrl_c().await {
                        Ok(()) => {
                            info!("🛑 Ctrl+C received - performing cleanup...");
                            
                            match perform_cleanup(&state_for_signal).await {
                                Ok(msg) => info!("✅ {}", msg),
                                Err(e) => info!("⚠️  Shutdown warning: {}", e),
                            }
                            
                            info!("🚪 Exiting app after Ctrl+C cleanup");
                            app_handle_for_signal.exit(0);
                        }
                        Err(e) => {
                            info!("⚠️  Error setting up Ctrl+C handler: {}", e);
                        }
                    }
                });
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent default close behavior
                api.prevent_close();
                
                // Perform cleanup and exit when window is closed
                let app_handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let tauri_state = app_handle.state::<TauriState>();
                    info!("🛑 Window close requested - performing cleanup...");

                    match perform_cleanup(&tauri_state).await {
                        Ok(msg) => {
                            info!("✅ {}", msg);
                        }
                        Err(e) => {
                            info!("⚠️  Shutdown warning: {}", e);
                        }
                    }

                    // Exit after cleanup
                    info!("🚪 Exiting app after cleanup");
                    app_handle.exit(0);
                });
            }
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            activate_eltord_invoke,
            deactivate_eltord_invoke,
            get_eltord_status_invoke,
            get_eltord_logs_invoke,
            stream_eltord_logs_invoke,
            stop_eltord_logs_invoke,
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
            stop_phoenix_daemon,
            detect_phoenix_config,
            update_relay_payment_rate
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
