use anyhow::{Context, Result};
use std::sync::Mutex;
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Listener, Manager, Runtime,
};

use crate::cache::{read_cache, SubscriptionData};

const TRAY_ID: &str = "claudeview";

const GREEN_PNG: &[u8] = include_bytes!("../icons/tray-green.png");
const ORANGE_PNG: &[u8] = include_bytes!("../icons/tray-orange.png");
const RED_PNG: &[u8] = include_bytes!("../icons/tray-red.png");

/// Color thresholds (PRA §3.1): green < 80%, orange 80-95%, red > 95%.
const ORANGE_THRESHOLD: f64 = 80.0;
const RED_THRESHOLD: f64 = 95.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TrayState {
    Green,
    Orange,
    Red,
}

impl TrayState {
    fn from_percent(p: Option<f64>) -> Self {
        match p {
            Some(p) if p > RED_THRESHOLD => TrayState::Red,
            Some(p) if p >= ORANGE_THRESHOLD => TrayState::Orange,
            _ => TrayState::Green,
        }
    }

    fn png(self) -> &'static [u8] {
        match self {
            TrayState::Green => GREEN_PNG,
            TrayState::Orange => ORANGE_PNG,
            TrayState::Red => RED_PNG,
        }
    }
}

/// Track the most recently-applied state so update_from_cache can skip
/// no-op redraws — desktop tray daemons sometimes flash on every set.
static LAST_STATE: Mutex<Option<TrayState>> = Mutex::new(None);

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let icon = Image::from_bytes(GREEN_PNG).context("decoding initial tray icon")?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("ClaudeView")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)
        .context("building tray icon")?;

    // Initial color from whatever is in the cache now.
    update_from_cache(app);

    // Re-color on every cache change. The watcher emits this event; here
    // we just react to it on the Rust side as well so the tray stays in
    // sync without the frontend having to drive it.
    let handle = app.clone();
    app.listen("cache-updated", move |_| {
        update_from_cache(&handle);
    });

    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        _ => {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn update_from_cache<R: Runtime>(app: &AppHandle<R>) {
    let percent = read_cache().ok().and_then(|c| max_used_percent(c.subscription.as_ref()));
    apply_state(app, TrayState::from_percent(percent));
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

fn apply_state<R: Runtime>(app: &AppHandle<R>, state: TrayState) {
    {
        let mut last = LAST_STATE.lock().expect("LAST_STATE poisoned");
        if *last == Some(state) {
            return;
        }
        *last = Some(state);
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match Image::from_bytes(state.png()) {
        Ok(image) => {
            let _ = tray.set_icon(Some(image));
        }
        Err(e) => eprintln!("[tray] decode icon for {state:?} failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{LimitData, SubscriptionData};

    fn limit(used: u64, total: u64) -> LimitData {
        LimitData {
            used: Some(used),
            total: Some(total),
            reset_at: None,
        }
    }

    #[test]
    fn percent_handles_missing_and_zero() {
        assert_eq!(percent(Some(50), Some(100)), Some(50.0));
        assert_eq!(percent(None, Some(100)), Some(0.0));
        assert_eq!(percent(Some(50), None), None);
        assert_eq!(percent(Some(50), Some(0)), None);
    }

    #[test]
    fn max_used_percent_picks_higher_window() {
        let sub = SubscriptionData {
            five_hour_limit: Some(limit(40, 100)),  // 40%
            seven_day_limit: Some(limit(700, 1000)), // 70%
        };
        assert_eq!(max_used_percent(Some(&sub)), Some(70.0));
    }

    #[test]
    fn state_thresholds() {
        assert_eq!(TrayState::from_percent(None), TrayState::Green);
        assert_eq!(TrayState::from_percent(Some(0.0)), TrayState::Green);
        assert_eq!(TrayState::from_percent(Some(79.9)), TrayState::Green);
        assert_eq!(TrayState::from_percent(Some(80.0)), TrayState::Orange);
        assert_eq!(TrayState::from_percent(Some(95.0)), TrayState::Orange);
        assert_eq!(TrayState::from_percent(Some(95.01)), TrayState::Red);
        assert_eq!(TrayState::from_percent(Some(100.0)), TrayState::Red);
    }
}
