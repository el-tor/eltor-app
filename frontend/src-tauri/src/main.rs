#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eltor_backend::eltor::EltorMode;
use eltor_backend::lightning::ListTransactionsParams;
use serde::Serialize;
use std::env;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{command, generate_context, AppHandle, Builder, Emitter, Manager, State, WindowEvent};

// Import backend library
use eltor_backend::{
    activate_eltord, create_app_state, deactivate_eltord, get_eltord_status, get_log_receiver,
    initialize_phoenixd, lightning, lookup_ip_location, ports, shutdown_cleanup, torrc_parser,
    AppState as BackendAppState, EltorStatus, IpLocationResponse, LogEntry, PathConfig,
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
    backend_state: Arc<BackendAppState>,
    log_listener_active: Arc<tokio::sync::Mutex<bool>>,
    lightning_node: Arc<tokio::sync::Mutex<Option<lightning::LightningNode>>>,
}

impl TauriState {
    fn new() -> Self {
        let backend_state = create_app_state(true); // Enable embedded phoenixd for Tauri
        Self {
            backend_state: Arc::new(backend_state),
            log_listener_active: Arc::new(tokio::sync::Mutex::new(false)),
            lightning_node: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    async fn initialize_phoenixd(&self) -> Result<(), String> {
        initialize_phoenixd((*self.backend_state).clone()).await
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
    torrc_file_name: String,
    mode: EltorMode,
) -> Result<String, String> {
    println!(
        "🚀 activate_eltord command called with torrc_file_name: {:?}, mode: {:?}",
        torrc_file_name, mode
    );

    let backend_state = tauri_state.backend_state.clone();

    println!(
        "🔧 Current working directory: {:?}",
        std::env::current_dir()
    );

    // Use backend wrapper function with mode parameter
    match activate_eltord((*backend_state).clone(), mode).await {
        Ok(message) => {
            println!("✅ Backend activation successful: {}", message);

            // Check if log listener is already active
            let mut listener_active = tauri_state.log_listener_active.lock().await;
            if !*listener_active {
                *listener_active = true;
                println!("📡 Starting new log listener task");

                // Set up single log listener for this activation
                let listener_flag = tauri_state.log_listener_active.clone();
                let backend_state_clone = (*backend_state).clone();
                tokio::spawn(async move {
                    let mut log_receiver = get_log_receiver(&backend_state_clone);
                    while let Ok(log_entry) = log_receiver.recv().await {
                        let tauri_log: TauriLogEntry = log_entry.into();
                        if let Err(e) = app_handle.emit("eltord-log", &tauri_log) {
                            println!("Failed to emit log event: {}", e);
                            break;
                        }
                    }

                    // Reset flag when listener stops
                    *listener_flag.lock().await = false;
                    println!("📡 Log listener task stopped");
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
) -> Result<String, String> {
    println!("deactivate_eltord command called");

    let backend_state = tauri_state.backend_state.clone();

    // Use backend wrapper function
    // TODO pass in mode from UX
    deactivate_eltord((*backend_state).clone(), EltorMode::Client).await
}

#[command]
async fn get_eltord_status_invoke(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    let backend_state = tauri_state.backend_state.clone();

    // Use backend wrapper function
    let status = get_eltord_status((*backend_state).clone()).await;

    // Convert backend logs to Tauri format
    let tauri_logs: Vec<TauriLogEntry> = status
        .recent_logs
        .into_iter()
        .map(|log| log.into())
        .collect();

    Ok(serde_json::json!({
        "running": status.running,
        "pid": status.pid,
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
    shutdown_cleanup((*backend_state).clone()).await?;

    Ok("App shutdown cleanup completed".to_string())
}

#[command]
async fn list_lightning_configs() -> Result<serde_json::Value, String> {
    // Parse the torrc file to get all lightning configurations
    let path_config = PathConfig::new()?;
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
    config: serde_json::Value,
) -> Result<String, String> {
    println!("🗑️  delete_lightning_config called with config: {}", config);

    let path_config = PathConfig::new()?;
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
            if let Err(e) = reinitialize_lightning_node(&tauri_state).await {
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
    config: serde_json::Value,
) -> Result<String, String> {
    println!("💾 upsert_lightning_config called with config: {}", config);

    let path_config = PathConfig::new()?;
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
                if let Err(e) = reinitialize_lightning_node(&tauri_state).await {
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
async fn reinitialize_lightning_node(tauri_state: &TauriState) -> Result<(), String> {
    let path_config = PathConfig::new()?;
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

    let _ = TrayIconBuilder::with_id("main-tray")
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
                    match activate_eltord(
                        tauri_state,
                        app_handle.clone(),
                        None,
                        Some("client".to_string()),
                    )
                    .await
                    {
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
                    match deactivate_eltord(tauri_state, app_handle.clone()).await {
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
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.webview_windows().values().next() {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app);

    Ok(())
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

    // Initialize torrc files before starting the app
    if let Ok(path_config) = PathConfig::new() {
        if let Err(e) = path_config.ensure_torrc_files() {
            eprintln!("⚠️  Failed to initialize torrc files: {}", e);
            eprintln!("   Continuing with startup...");
        }
    } else {
        eprintln!("⚠️  Failed to get path configuration, continuing with startup...");
    }

    Builder::default()
        .setup(|app| {
            setup_tray(app.handle())?;

            // Initialize the Tauri state
            let tauri_state = TauriState::new();

            // Initialize phoenixd asynchronously after the runtime is available
            let app_handle = app.handle().clone();
            let state_for_init = tauri_state.clone();
            let backend_state = tauri_state.backend_state.clone();

            tauri::async_runtime::spawn(async move {
                // Clean up any processes using our ports
                println!("🧹 Starting port cleanup...");
                if let Err(e) = ports::cleanup_ports(&*backend_state).await {
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
                let torrc_path = match PathConfig::new() {
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
                let ip_db_path = match PathConfig::new() {
                    Ok(path_config) => path_config.get_executable_path("IP2LOCATION-LITE-DB3.BIN"),
                    Err(e) => {
                        eprintln!("⚠️  Failed to get path config: {}", e);
                        return;
                    }
                };
                if ip_db_path.exists() {
                    if let Err(e) = eltor_backend::init_ip_database(ip_db_path) {
                        eprintln!("⚠️  Failed to initialize IP database: {}", e);
                    }
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
        .plugin(tauri_plugin_log::Builder::default().build())
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
            upsert_lightning_config
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
