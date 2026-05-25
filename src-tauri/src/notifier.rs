use std::sync::Mutex;
use tauri::{AppHandle, Listener, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::cache::{read_cache, SubscriptionData};
use crate::config;

/// Last computed usage % (max of 5h / 7d), kept across cache-updated events
/// so we can fire only on *upward* crossings of configured thresholds.
/// Without this we would spam a notification on every PostToolUse write.
static LAST_PERCENT: Mutex<Option<f64>> = Mutex::new(None);

pub fn setup<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    app.listen("cache-updated", move |_| {
        maybe_notify(&handle);
    });
}

fn maybe_notify<R: Runtime>(app: &AppHandle<R>) {
    let prefs = config::load().unwrap_or_default();
    if !prefs.behavior.notifications_enabled {
        return;
    }

    let current = current_percent();

    let mut last = LAST_PERCENT.lock().expect("LAST_PERCENT poisoned");
    let previous = *last;
    *last = current;

    let (Some(prev), Some(cur)) = (previous, current) else {
        // Either first event of the session or no usage data — record state
        // and skip; we only fire on transitions.
        return;
    };

    for &threshold in &prefs.behavior.notification_thresholds {
        let t = threshold as f64;
        if prev < t && cur >= t {
            fire(app, threshold, cur);
        }
    }
}

fn current_percent() -> Option<f64> {
    let cache = read_cache().ok()?;
    max_used_percent(cache.subscription.as_ref())
}

fn max_used_percent(sub: Option<&SubscriptionData>) -> Option<f64> {
    let sub = sub?;
    let five = sub
        .five_hour_limit
        .as_ref()
        .and_then(|l| percent(l.used, l.total));
    let seven = sub
        .seven_day_limit
        .as_ref()
        .and_then(|l| percent(l.used, l.total));
    match (five, seven) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

fn percent(used: Option<u64>, total: Option<u64>) -> Option<f64> {
    let total = total?;
    if total == 0 {
        return None;
    }
    Some(used.unwrap_or(0) as f64 * 100.0 / total as f64)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_math_matches_tray_logic() {
        // Same intent as tray::tests::* — duplicated rather than shared to
        // keep each module self-contained.
        assert_eq!(percent(Some(80), Some(100)), Some(80.0));
        assert_eq!(percent(Some(50), Some(0)), None);
        assert_eq!(percent(Some(50), None), None);
    }
}
