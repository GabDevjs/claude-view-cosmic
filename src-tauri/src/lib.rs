mod collector;
mod config;
mod notifier;
mod poller;
mod tray;
mod watcher;

use config::Preferences;
use poller::SourceStatus;
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
fn get_sources_status() -> Vec<SourceStatus> {
    poller::get_status()
}

/// Force an immediate poll of every source (local + remotes).
#[tauri::command]
fn refresh_now() {
    poller::nudge();
}

#[tauri::command]
fn get_preferences() -> Result<Preferences, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_preferences(prefs: Preferences) -> Result<(), String> {
    config::save(&prefs).map_err(|e| e.to_string())?;
    // Pick up new hosts / interval / window on the next tick, immediately.
    poller::nudge();
    Ok(())
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
        .plugin(tauri_plugin_notification::init())
        // A tray app must survive its window closing: hide instead of quitting,
        // so the panel item stays and the window reopens on the next tray click.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            // Watcher nudges the poller on local cache changes for <1s updates.
            if let Err(e) = watcher::start() {
                eprintln!("[setup] watcher failed to start: {e}");
            }
            // Tray registers its SNI item; set up before the poller so its nudge
            // channel exists when the first poll cycle calls set_percent.
            if let Err(e) = tray::setup(app.handle()) {
                eprintln!("[setup] tray failed to start: {e}");
            }
            // Poller owns the refresh loop: fetch sources, update UI + tray.
            poller::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sources_status,
            refresh_now,
            get_preferences,
            set_preferences,
            toggle_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
