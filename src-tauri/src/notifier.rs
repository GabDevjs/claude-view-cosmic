//! Desktop notifications on upward crossings of configured usage thresholds.
//!
//! The poller feeds us the worst 5-hour usage across sources each cycle; we fire
//! only when a threshold is newly crossed upward, so we never spam on every
//! refresh.

use std::sync::Mutex;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::config::Preferences;

/// Last usage % seen, so we can fire only on *upward* crossings.
static LAST_PERCENT: Mutex<Option<f64>> = Mutex::new(None);

/// Called by the poller with the worst 5-hour usage across all sources.
pub fn on_percent<R: Runtime>(app: &AppHandle<R>, current: Option<f64>, prefs: &Preferences) {
    let mut last = LAST_PERCENT.lock().expect("LAST_PERCENT poisoned");
    let previous = *last;
    *last = current;

    if !prefs.behavior.notifications_enabled {
        // Keep LAST_PERCENT current so re-enabling doesn't fire on stale deltas.
        return;
    }

    let (Some(prev), Some(cur)) = (previous, current) else {
        // First reading of the session or no usage data — record and skip; we
        // only fire on transitions.
        return;
    };

    for &threshold in &prefs.behavior.notification_thresholds {
        let t = threshold as f64;
        if prev < t && cur >= t {
            fire(app, threshold, cur);
        }
    }
}

fn fire<R: Runtime>(app: &AppHandle<R>, threshold: u32, current: f64) {
    let body = format!("Claude Code usage at {current:.0}% (crossed {threshold}%)");
    let result = app
        .notification()
        .builder()
        .title("ClaudeView")
        .body(body)
        .show();
    if let Err(e) = result {
        eprintln!("[notifier] show failed: {e}");
    }
}
