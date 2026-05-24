mod cache;
mod sessions;
mod transcripts;
mod watcher;

use cache::{read_cache, SubscriptionData};
use sessions::Session;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(e) = watcher::start(app.handle().clone()) {
                eprintln!("[setup] watcher failed to start: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_subscription_data,
            get_active_sessions,
            get_token_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
