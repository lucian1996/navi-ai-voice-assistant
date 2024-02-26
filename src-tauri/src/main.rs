// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app::AppState;
use app::PlaybackCommand;
use app::UserSettings;
use app::_utils::_api::azure::speak_text;
use app::_utils::_api::ollama::speak_ollama;
use app::_utils::playback;
use app::_utils::user_settings::get_user_settings_path;
use app::_utils::user_settings::load_user_settings;
use std::sync::Arc;
use tauri::Manager;
use tauri::SystemTray;
use tauri::SystemTrayEvent;
use tauri::{CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};
use tokio::sync::Mutex;
use tokio::task;

#[tokio::main]
async fn main() {
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_item(quit);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    let playback_send = playback::init_playback_channel().await;

    let user_settings = match load_user_settings() {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("Failed to load user settings: {}", err);
            UserSettings::default()
        }
    };

    let nexus = Arc::new(Mutex::new(AppState {
        playback_send: playback_send.clone(),
        user_settings: Some(user_settings),
        user_array: Vec::new(),
        user_messages_array: Vec::new(),
    }));

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| handle_system_tray_event(app, event))
        .invoke_handler(tauri::generate_handler![
            speak_text_from_frontend,
            speak_ollama_from_frontend,
            pause_playback_from_frontend,
            resume_playback_from_frontend,
            stop_playback_from_frontend,
            fast_forward_playback_from_frontend,
            get_user_settings_as_json
        ])
        .manage(nexus.clone())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
}

// region: --- Main Commands

use serde_json;

#[tauri::command]
async fn get_user_settings_as_json(
    nexus: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let nexus_lock = nexus.lock().await;
    if let Some(user_settings) = &nexus_lock.user_settings {
        match serde_json::to_string(user_settings) {
            Ok(json) => Ok(json),
            Err(err) => Err(format!("Failed to convert user settings to JSON: {}", err)),
        }
    } else {
        Err("User settings not found.".to_string())
    }
}

use serde_json::Value;

#[tauri::command]
async fn set_user_settings_from_json(
    nexus: tauri::State<'_, Arc<Mutex<AppState>>>,
    settings_json: String,
) -> Result<(), String> {
    let mut nexus_lock = nexus.lock().await;
    let settings_value: Value = match serde_json::from_str(&settings_json) {
        Ok(value) => value,
        Err(err) => return Err(format!("Failed to parse JSON: {}", err)),
    };

    // Assuming your AppState struct has a field called user_settings of type Option<UserSettings>
    if let Some(user_settings) = &mut nexus_lock.user_settings {
        *user_settings = match serde_json::from_value(settings_value) {
            Ok(settings) => settings,
            Err(err) => return Err(format!("Failed to convert JSON to user settings: {}", err)),
        };
        Ok(())
    } else {
        Err("User settings not found.".to_string())
    }
}

#[tauri::command]
async fn speak_text_from_frontend(text: String, app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move {
        speak_text(&text, &playback_send).await;
    });
    Ok(())
}

#[tauri::command]
async fn speak_ollama_from_frontend(prompt: String, app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move {
        speak_ollama(prompt, &playback_send).await;
    });
    Ok(())
}

// endregion: --- Main Commands

// region: --- Playback Commands

#[tauri::command]
async fn pause_playback_from_frontend(app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move { playback_send.send(PlaybackCommand::Pause).await });
    Ok(())
}

#[tauri::command]
async fn resume_playback_from_frontend(app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move { playback_send.send(PlaybackCommand::Resume).await });
    Ok(())
}

#[tauri::command]
async fn stop_playback_from_frontend(app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move { playback_send.send(PlaybackCommand::Stop).await });
    Ok(())
}

#[tauri::command]
async fn fast_forward_playback_from_frontend(app: tauri::AppHandle) -> Result<(), String> {
    let playback_send = {
        let nexus_lock = app.state::<Arc<Mutex<AppState>>>();
        let nexus = nexus_lock.lock().await;
        nexus.playback_send.clone()
    };
    task::spawn(async move { playback_send.send(PlaybackCommand::FastForward).await });
    Ok(())
}

// endregion: --- Playback Commands

fn handle_system_tray_event(app: &tauri::AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick {
            position: _,
            size: _,
            ..
        } => {
            println!("system tray received a left click");
        }
        SystemTrayEvent::RightClick {
            position: _,
            size: _,
            ..
        } => {
            println!("system tray received a right click");
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                std::process::exit(0);
            }
            "show" => {
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
            }
            "hide" => {
                let window = app.get_window("main").unwrap();
                window.hide().unwrap();
            }
            _ => {}
        },
        _ => {}
    }
}
