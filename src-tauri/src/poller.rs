//! Background poller: the always-alive heart of the app.
//!
//! On its own thread it reloads preferences, fetches every configured source
//! (local + remote hosts) *concurrently*, stores the results, and pushes a
//! `sources-updated` event to the UI. It also drives the tray color and the
//! threshold notifier from the worst 5-hour usage across sources — so both stay
//! correct even when the window is hidden and its webview timers are throttled.
//!
//! This mirrors the macOS status-bar controller, which owns the refresh loop
//! rather than delegating it to the popover view.

use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::collector::{self, SourceConfig, SourceSnapshot};
use crate::config;
use crate::{notifier, tray};

pub const EVENT: &str = "sources-updated";

/// Per-source fetch outcome. Serialized as `{status, data}` so the frontend can
/// discriminate: `{status:"ok", data: <snapshot>}`, `{status:"error", data:
/// "<msg>"}`, or `{status:"loading"}`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
pub enum SourceState {
    Loading,
    Ok(Box<SourceSnapshot>),
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub config: SourceConfig,
    pub state: SourceState,
}

/// Current status of every source. Cloned out for `get_sources_status`.
static STATE: Mutex<Vec<SourceStatus>> = Mutex::new(Vec::new());

/// Wakes the poll loop early (prefs changed, local cache touched).
static NUDGE: OnceLock<mpsc::Sender<()>> = OnceLock::new();

/// Per-source fetch timeout. SSH's own ConnectTimeout bounds a dead host sooner.
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const MIN_INTERVAL_SECS: u64 = 5;

pub fn start<R: Runtime>(app: AppHandle<R>) {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = NUDGE.set(tx);

    std::thread::Builder::new()
        .name("claudeview-poller".into())
        .spawn(move || loop {
            run_cycle(&app);
            let interval = poll_interval();
            // Sleep until the interval elapses or someone nudges us.
            let _ = rx.recv_timeout(interval);
            // Coalesce a burst of nudges into a single extra cycle.
            while rx.try_recv().is_ok() {}
        })
        .expect("spawn claudeview-poller thread");
}

/// Wake the poller now (called on prefs save and on local cache changes).
pub fn nudge() {
    if let Some(tx) = NUDGE.get() {
        let _ = tx.send(());
    }
}

pub fn get_status() -> Vec<SourceStatus> {
    STATE.lock().expect("poller STATE poisoned").clone()
}

fn poll_interval() -> Duration {
    let secs = config::load()
        .map(|p| p.display.refresh_interval_seconds as u64)
        .unwrap_or(60)
        .max(MIN_INTERVAL_SECS);
    Duration::from_secs(secs)
}

fn run_cycle<R: Runtime>(app: &AppHandle<R>) {
    let prefs = config::load().unwrap_or_default();
    let window_days = prefs.display.history_window_days;
    let configs = prefs.sources();

    reconcile(&configs);
    let _ = app.emit(EVENT, ());

    // Fetch every source concurrently so one slow/dead remote never delays the
    // local refresh. Each thread writes its own slot and emits on completion.
    let handles: Vec<_> = configs
        .into_iter()
        .map(|cfg| {
            let app = app.clone();
            std::thread::spawn(move || {
                let state = match collector::fetch(cfg.ssh_host.as_deref(), window_days, FETCH_TIMEOUT)
                {
                    Ok(snapshot) => SourceState::Ok(Box::new(snapshot)),
                    Err(e) => SourceState::Error(e.to_string()),
                };
                set_state(&cfg, state);
                let _ = app.emit(EVENT, ());
            })
        })
        .collect();
    for handle in handles {
        let _ = handle.join();
    }

    // Tray text mirrors the LOCAL machine's statusLine (5h/7d); the notifier
    // still fires on the worst 5-hour usage across all sources.
    let (local_five, local_seven) = local_quota();
    tray::set_quota(local_five, local_seven);
    notifier::on_percent(app, worst_five_hour(), &prefs);
}

/// Rebuild STATE for the current source list, preserving prior snapshots and
/// marking brand-new sources as Loading (so the UI shows a spinner, not blank).
fn reconcile(configs: &[SourceConfig]) {
    let mut state = STATE.lock().expect("poller STATE poisoned");
    let previous = std::mem::take(&mut *state);
    *state = configs
        .iter()
        .map(|cfg| {
            let prior = previous
                .iter()
                .find(|s| s.config.key() == cfg.key())
                .map(|s| s.state.clone());
            SourceStatus {
                config: cfg.clone(),
                state: prior.unwrap_or(SourceState::Loading),
            }
        })
        .collect();
}

fn set_state(cfg: &SourceConfig, new_state: SourceState) {
    let mut state = STATE.lock().expect("poller STATE poisoned");
    if let Some(slot) = state.iter_mut().find(|s| s.config.key() == cfg.key()) {
        slot.state = new_state;
    }
}

fn worst_five_hour() -> Option<f64> {
    let state = STATE.lock().expect("poller STATE poisoned");
    state
        .iter()
        .filter_map(|s| match &s.state {
            SourceState::Ok(snapshot) => collector::five_hour_pct(snapshot),
            _ => None,
        })
        .fold(None, |acc, pct| Some(acc.map_or(pct, |a: f64| a.max(pct))))
}

/// The local machine's (5h, 7d) usage %, for the tray text.
fn local_quota() -> (Option<f64>, Option<f64>) {
    let state = STATE.lock().expect("poller STATE poisoned");
    for status in state.iter() {
        if status.config.ssh_host.is_none() {
            if let SourceState::Ok(snapshot) = &status.state {
                if let Some(q) = &snapshot.quota {
                    return (q.five_hour_pct, q.seven_day_pct);
                }
            }
            break;
        }
    }
    (None, None)
}
