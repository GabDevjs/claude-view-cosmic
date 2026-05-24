use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::thread;
use tauri::{AppHandle, Emitter};

use crate::cache::cache_path;

pub const EVENT: &str = "cache-updated";

/// Start watching ~/.claude/claudeview_cache.json. Emits `cache-updated`
/// (no payload) on every modify/create so the frontend can re-fetch via
/// `get_subscription_data`.
///
/// Watches the *parent directory* rather than the file itself so creation
/// events still fire when the cache does not exist yet (first run).
pub fn start(app: AppHandle) -> Result<()> {
    let target = cache_path()?;
    let watch_dir = target
        .parent()
        .context("cache path has no parent")?
        .to_path_buf();

    // ~/.claude almost always exists after the user has run Claude Code,
    // but create it defensively so the watcher init never fails on a
    // fresh machine.
    std::fs::create_dir_all(&watch_dir).ok();

    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher: RecommendedWatcher = match Watcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[watcher] init failed: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            eprintln!("[watcher] watch({}) failed: {e}", watch_dir.display());
            return;
        }

        // Fire once at startup so the UI populates even if no file event
        // occurs after launch.
        let _ = app.emit(EVENT, ());

        for res in rx {
            match res {
                Ok(event) => {
                    let touches_target = event.paths.iter().any(|p| p == &target);
                    let relevant = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );
                    if touches_target && relevant {
                        let _ = app.emit(EVENT, ());
                    }
                }
                Err(e) => eprintln!("[watcher] event err: {e}"),
            }
        }
    });

    Ok(())
}
