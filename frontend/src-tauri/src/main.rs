#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eltor_backend::lightning::ListTransactionsParams;
use serde::Serialize;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{command, generate_context, AppHandle, Builder, Emitter, Manager, State, WindowEvent};

// Import backend library
use eltor_backend::{
    activate_eltord_wrapper, create_app_state, deactivate_eltord_wrapper, get_bin_dir,
    get_eltord_status_wrapper, get_log_receiver, initialize_phoenixd, lightning, ports,
    AppState as BackendAppState, LogEntry,
};

// Tauri-specific log entry format for frontend compatibility
#[derive(Debug, Clone, Serialize)]
struct TauriLogEntry {
    timestamp: String,
    level: String,
    message: String,
    source: String,
}

impl From<LogEntry> for TauriLogEntry {
    fn from(log: LogEntry) -> Self {
        Self {
            timestamp: log.timestamp.to_rfc3339(),
            level: log.level,
            message: log.message,
            source: log.source,
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
async fn connect_tor() -> Result<String, String> {
    println!("Connecting to Tor...");
    Ok("Connected to Tor".to_string())
}

#[command]
async fn disconnect_tor() -> Result<String, String> {
    println!("Disconnecting from Tor...");
    Ok("Disconnected from Tor".to_string())
}

#[command]
async fn get_tor_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "connected": false,
        "circuit": null
    }))
}

#[command]
async fn activate_eltord(
    tauri_state: State<'_, TauriState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    println!("🚀 activate_eltord command called");

    let backend_state = tauri_state.backend_state.clone();

    println!(
        "🔧 Current working directory: {:?}",
        std::env::current_dir()
    );

    // Use backend wrapper function
    match activate_eltord_wrapper((*backend_state).clone()).await {
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
async fn deactivate_eltord(
    tauri_state: State<'_, TauriState>,
    _app_handle: AppHandle,
) -> Result<String, String> {
    println!("deactivate_eltord command called");

    let backend_state = tauri_state.backend_state.clone();

    // Use backend wrapper function
    deactivate_eltord_wrapper((*backend_state).clone()).await
}

#[command]
async fn get_eltord_status(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    let backend_state = tauri_state.backend_state.clone();

    // Use backend wrapper function
    let status = get_eltord_status_wrapper((*backend_state).clone()).await;

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
async fn get_wallet_balance(
    tauri_state: State<'_, TauriState>,
) -> Result<serde_json::Value, String> {
    // Get the lightning node from TauriState
    let lightning_node_guard = tauri_state.lightning_node.lock().await;
    let lightning_node = lightning_node_guard
        .as_ref()
        .ok_or("Lightning node not initialized")?;

    // Get the balance from the lightning node
    match lightning_node.get_balance().await {
        Ok(balance) => {
            println!("✅ Retrieved wallet balance: {} sats", balance.confirmed_balance_sats);
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
    };
    
    match lightning_node.list_transactions(params).await {
        Ok(transactions) => Ok(serde_json::json!(transactions)),
        Err(e) => {
            println!("❌ Failed to get wallet txns: {}", e);
            Err(format!("Failed to get wallet txns: {}", e))
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
                app.exit(0);
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
                    match activate_eltord(tauri_state, app_handle.clone()).await {
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
    Builder::default()
        .setup(|app| {
            setup_tray(app.handle())?;

            // Initialize the Tauri state
            let tauri_state = TauriState::new();

            // Initialize phoenixd asynchronously after the runtime is available
            let app_handle = app.handle().clone();
            let state_for_init = tauri_state.clone();

            tauri::async_runtime::spawn(async move {
                // Clean up any processes using our ports
                println!("🧹 Starting port cleanup...");
                if let Err(e) = ports::cleanup_ports().await {
                    eprintln!("⚠️  Port cleanup failed: {}", e);
                    eprintln!("   Continuing with startup...");
                }

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

                // Initialize lightning node
                let torrc_path = get_bin_dir().join("torrc");
                match lightning::LightningNode::from_torrc(torrc_path) {
                    Ok(node) => {
                        println!("✅ Lightning node connected from torrc ({})", node.node_type());
                        // Store the lightning node in TauriState
                        let mut lightning_node_guard = state_for_init.lightning_node.lock().await;
                        *lightning_node_guard = Some(node);
                        drop(lightning_node_guard); // Release the lock
                        let _ = app_handle.emit("lightning-ready", "Lightning node ready");
                    }
                    Err(e) => {
                        println!("⚠️  Failed to initialize Lightning node from torrc: {}", e);
                        let _ = app_handle.emit("lightning-error", format!("Lightning initialization failed: {}", e));
                    }
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
            connect_tor,
            disconnect_tor,
            get_tor_status,
            activate_eltord,
            deactivate_eltord,
            get_eltord_status,
            test_log_event,
            get_wallet_balance,
            get_wallet_transactions
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
