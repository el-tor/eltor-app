#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, command, generate_context, Builder, Manager, WindowEvent, Emitter, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use std::process::{Command, Child};
use std::sync::Mutex;

// Global state to track the eltord process
struct EltordProcess(Mutex<Option<Child>>);

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
async fn activate_eltord(eltord_state: State<'_, EltordProcess>) -> Result<String, String> {
    // Check if process is already running
    {
        let mut process_guard = eltord_state.0.lock().unwrap();
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
    
    let child = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("client")
        .arg("-f")
        .arg("torrc.client.prod")
        .arg("-pw")
        .arg("password1234_")
        .current_dir(&eltord_path)
        .spawn();
    
    match child {
        Ok(process) => {
            let pid = process.id();
            println!("Eltord process started with PID: {}", pid);
            
            // Store the process
            let mut process_guard = eltord_state.0.lock().unwrap();
            *process_guard = Some(process);
            
            Ok(format!("Eltord activated with PID: {}", pid))
        }
        Err(e) => {
            let error_msg = format!("Failed to start eltord: {}", e);
            println!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[command]
async fn deactivate_eltord(eltord_state: State<'_, EltordProcess>) -> Result<String, String> {
    let mut process_guard = eltord_state.0.lock().unwrap();
    
    if let Some(mut child) = process_guard.take() {
        match child.kill() {
            Ok(_) => {
                println!("Eltord process killed successfully");
                // Wait for the process to actually terminate
                let _ = child.wait();
                Ok("Eltord deactivated successfully".to_string())
            }
            Err(e) => {
                let error_msg = format!("Failed to kill eltord process: {}", e);
                println!("{}", error_msg);
                Err(error_msg)
            }
        }
    } else {
        Err("No eltord process is currently running".to_string())
    }
}

#[command]
async fn get_eltord_status(eltord_state: State<'_, EltordProcess>) -> Result<serde_json::Value, String> {
    let mut process_guard = eltord_state.0.lock().unwrap();
    
    let is_running = if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process has exited
                *process_guard = None;
                false
            }
            Ok(None) => {
                // Process is still running
                true
            }
            Err(_) => {
                // Error checking process, assume it's dead
                *process_guard = None;
                false
            }
        }
    } else {
        false
    };

    Ok(serde_json::json!({
        "running": is_running,
        "pid": if is_running { 
            process_guard.as_ref().map(|p| p.id()) 
        } else { 
            None 
        }
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
                    match activate_eltord(eltord_state).await {
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
                    match deactivate_eltord(eltord_state).await {
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
        .manage(EltordProcess(Mutex::new(None)))  // Add state management
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            connect_tor,
            disconnect_tor,
            get_tor_status,
            activate_eltord,
            deactivate_eltord,
            get_eltord_status
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}