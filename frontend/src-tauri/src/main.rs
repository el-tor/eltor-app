#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, command, generate_context, Builder, Manager, WindowEvent, Emitter, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
    source: String, // "stdout" or "stderr"
}

// Global state to track the eltord process and logs
struct EltordProcess {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    recent_logs: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl EltordProcess {
    fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            recent_logs: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
        }
    }

    fn add_log(&self, entry: LogEntry) {
        let mut logs = self.recent_logs.lock().unwrap();
        if logs.len() >= 100 {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    fn get_recent_logs(&self) -> Vec<LogEntry> {
        self.recent_logs.lock().unwrap().clone().into()
    }
}

// Function to read logs from a process stream and emit events
async fn read_process_logs(
    mut reader: AsyncBufReader<tokio::process::ChildStdout>,
    state: Arc<EltordProcess>,
    app_handle: AppHandle,
    source: &'static str,
) {
    let mut line = String::new();
    let mut line_count = 0;
    println!("🚀 Starting to read {} logs", source);
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("📄 EOF reached for {} after {} lines", source, line_count);
                break; // EOF
            }
            Ok(bytes_read) => {
                line_count += 1;
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let entry = LogEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        level: "INFO".to_string(),
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[{}:{}] {} bytes: {}", source, line_count, bytes_read, trimmed);
                    state.add_log(entry.clone());
                    
                    // Emit log event to frontend
                    match app_handle.emit("eltord-log", &entry) {
                        Ok(_) => println!("✅ Successfully emitted eltord-log event #{} for {}", line_count, source),
                        Err(e) => println!("❌ Failed to emit eltord-log event #{} for {}: {}", line_count, source, e),
                    }
                } else {
                    println!("[{}:{}] Empty line skipped", source, line_count);
                }
            }
            Err(e) => {
                println!("❌ Error reading {} logs at line {}: {}", source, line_count, e);
                break;
            }
        }
    }
    println!("🏁 Finished reading {} logs. Total lines processed: {}", source, line_count);
}

// Function to read stderr logs
async fn read_process_stderr_logs(
    mut reader: AsyncBufReader<tokio::process::ChildStderr>,
    state: Arc<EltordProcess>,
    app_handle: AppHandle,
    source: &'static str,
) {
    let mut line = String::new();
    let mut line_count = 0;
    println!("🚀 Starting to read {} logs", source);
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("📄 EOF reached for {} after {} lines", source, line_count);
                break; // EOF
            }
            Ok(bytes_read) => {
                line_count += 1;
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let entry = LogEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        level: "ERROR".to_string(),
                        message: trimmed.to_string(),
                        source: source.to_string(),
                    };
                    
                    println!("[{}:{}] {} bytes: {}", source, line_count, bytes_read, trimmed);
                    state.add_log(entry.clone());
                    
                    // Emit log event to frontend
                    match app_handle.emit("eltord-log", &entry) {
                        Ok(_) => println!("✅ Successfully emitted eltord-log event #{} for {}", line_count, source),
                        Err(e) => println!("❌ Failed to emit eltord-log event #{} for {}: {}", line_count, source, e),
                    }
                } else {
                    println!("[{}:{}] Empty line skipped", source, line_count);
                }
            }
            Err(e) => {
                println!("❌ Error reading {} logs at line {}: {}", source, line_count, e);
                break;
            }
        }
    }
    println!("🏁 Finished reading {} logs. Total lines processed: {}", source, line_count);
}

#[command]
async fn test_log_event(app_handle: AppHandle) -> Result<String, String> {
    println!("test_log_event command called");
    
    let test_log = LogEntry {
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
    eltord_state: State<'_, EltordProcess>,
    app_handle: AppHandle,
) -> Result<String, String> {
    println!("activate_eltord command called");
    
    // Check if process is already running
    {
        let mut process_guard = eltord_state.process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited, clear it
                    *process_guard = None;
                }
                Ok(None) => {
                    // Process is still running
                    return Err("Eltord is already running".to_string());
                }
                Err(_) => {
                    // Error checking process, assume it's dead
                    *process_guard = None;
                }
            }
        }
    }

    let eltord_path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join("code/eltord");
    
    println!("Running eltord from: {:?}", eltord_path);
    
    let mut child = TokioCommand::new("cargo")
        .arg("run")
        .arg("--")
        .arg("client")
        .arg("-f")
        .arg("torrc.client.prod")
        .arg("-pw")
        .arg("password1234_")
        .current_dir(&eltord_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start eltord: {}", e))?;
    
    let pid = child.id().unwrap_or(0);
    println!("Eltord process started with PID: {}", pid);
    
    // Set up log readers
    if let Some(stdout) = child.stdout.take() {
        let reader = AsyncBufReader::new(stdout);
        let state_clone = Arc::new(EltordProcess {
            process: eltord_state.process.clone(),
            recent_logs: eltord_state.recent_logs.clone(),
        });
        let app_handle_clone = app_handle.clone();
        tokio::spawn(async move {
            read_process_logs(reader, state_clone, app_handle_clone, "stdout").await;
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let reader = AsyncBufReader::new(stderr);
        let state_clone = Arc::new(EltordProcess {
            process: eltord_state.process.clone(),
            recent_logs: eltord_state.recent_logs.clone(),
        });
        let app_handle_clone = app_handle.clone();
        tokio::spawn(async move {
            read_process_stderr_logs(reader, state_clone, app_handle_clone, "stderr").await;
        });
    }
    
    // Store the process
    {
        let mut process_guard = eltord_state.process.lock().unwrap();
        *process_guard = Some(child);
    }
    
    // Add startup log
    let startup_log = LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        message: format!("Eltord process started with PID: {}", pid),
        source: "system".to_string(),
    };
    eltord_state.add_log(startup_log.clone());
    let _ = app_handle.emit("eltord-log", &startup_log);
    
    Ok(format!("Eltord activated with PID: {}", pid))
}

#[command]
async fn deactivate_eltord(
    eltord_state: State<'_, EltordProcess>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Take the process out of the mutex first to avoid holding the lock across await
    let mut child = {
        let mut process_guard = eltord_state.process.lock().unwrap();
        process_guard.take()
    };
    
    if let Some(ref mut child) = child {
        match child.kill().await {
            Ok(_) => {
                println!("🛑 Killing eltord process...");
                let _ = child.wait().await; // Wait for process to actually terminate
                println!("✅ Eltord process terminated successfully");
                
                // Add shutdown log
                let shutdown_log = LogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "INFO".to_string(),
                    message: "Eltord process terminated".to_string(),
                    source: "system".to_string(),
                };
                eltord_state.add_log(shutdown_log.clone());
                let _ = app_handle.emit("eltord-log", &shutdown_log);
                
                Ok("Eltord deactivated successfully".to_string())
            }
            Err(e) => {
                Err(format!("Failed to kill eltord process: {}", e))
            }
        }
    } else {
        Err("No eltord process is currently running".to_string())
    }
}

#[command]
async fn get_eltord_status(eltord_state: State<'_, EltordProcess>) -> Result<serde_json::Value, String> {
    let mut process_guard = eltord_state.process.lock().unwrap();
    
    let (running, pid) = if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited
                *process_guard = None;
                (false, None)
            }
            Ok(None) => {
                // Process is still running
                (true, child.id())
            }
            Err(_) => {
                // Error checking process, assume it's dead
                *process_guard = None;
                (false, None)
            }
        }
    } else {
        (false, None)
    };

    let recent_logs = eltord_state.get_recent_logs();

    Ok(serde_json::json!({
        "running": running,
        "pid": pid,
        "recent_logs": recent_logs
    }))
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let activate_i = MenuItem::with_id(app, "activate", "Activate", true, None::<&str>)?;
    let deactivate_i = MenuItem::with_id(app, "deactivate", "Deactivate", true, None::<&str>)?;
    
    let menu = Menu::with_items(app, &[&show_i, &hide_i, &activate_i, &deactivate_i, &quit_i])?;
    
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
                    let eltord_state = app_handle.state::<EltordProcess>();
                    match activate_eltord(eltord_state, app_handle.clone()).await {
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
                    let eltord_state = app_handle.state::<EltordProcess>();
                    match deactivate_eltord(eltord_state, app_handle.clone()).await {
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
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .manage(EltordProcess::new())  // Add state management
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            connect_tor,
            disconnect_tor,
            get_tor_status,
            activate_eltord,
            deactivate_eltord,
            get_eltord_status,
            test_log_event
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}