mod cache;
mod config;
mod sessions;
mod transcripts;
mod tray;
mod watcher;

use cache::{read_cache, SubscriptionData};
use config::Preferences;
use sessions::Session;
use tauri_plugin_autostart::ManagerExt;
use transcripts::TokenStats;

#[tauri::command]
fn get_active_sessions() -> Vec<Session> {
    sessions::scan()
}

#[tauri::command]
fn get_token_stats(days: u32) -> Result<TokenStats, String> {
    transcripts::aggregate(days).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_subscription_data() -> Result<Option<SubscriptionData>, String> {
    match read_cache() {
        Ok(data) => Ok(data.subscription),
        Err(e) => {
            // Missing cache file is the normal "not yet populated" state —
            // surface it as Ok(None) so the UI can render an empty view
            // instead of an error.
            if let Some(io) = e.downcast_ref::<std::io::Error>() {
                if io.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                }
            }
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn get_preferences() -> Result<Preferences, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_preferences(prefs: Preferences) -> Result<(), String> {
    config::save(&prefs).map_err(|e| e.to_string())
}

/// Enable/disable XDG autostart AND persist the flag, so the next launch
/// reflects what the user toggled. Frontend should call this — not
/// `set_preferences` — for the autostart toggle so the system state stays
/// in sync with the file.
#[tauri::command]
fn toggle_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enable {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    let mut prefs = config::load().map_err(|e| e.to_string())?;
    prefs.behavior.autostart = enable;
    config::save(&prefs).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            if let Err(e) = watcher::start(app.handle().clone()) {
                eprintln!("[setup] watcher failed to start: {e}");
            }
            if let Err(e) = tray::setup(app.handle()) {
                eprintln!("[setup] tray failed to start: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_subscription_data,
            get_active_sessions,
            get_token_stats,
            get_preferences,
            set_preferences,
            toggle_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
