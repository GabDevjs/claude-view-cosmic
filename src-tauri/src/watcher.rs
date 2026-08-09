//! Watches `~/.claude/claudeview_cache.json` so a local Claude Code interaction
//! refreshes the UI in <1s instead of waiting for the next poll tick. On any
//! change it just nudges the poller, which re-fetches every source.

use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;

use crate::poller;

fn cache_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("$HOME is not set")?;
    Ok(home.join(".claude").join("claudeview_cache.json"))
}

/// Start watching the local quota cache. Watches the *parent directory* rather
/// than the file itself so a first-run create still fires.
pub fn start() -> Result<()> {
    let target = cache_path()?;
    let watch_dir = target
        .parent()
        .context("cache path has no parent")?
        .to_path_buf();

    // ~/.claude usually exists once the user has run Claude Code, but create it
    // defensively so watcher init never fails on a fresh machine.
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

        for res in rx {
            match res {
                Ok(event) => {
                    let touches_target = event.paths.iter().any(|p| p == &target);
                    let relevant = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );
                    if touches_target && relevant {
                        poller::nudge();
                    }
                }
                Err(e) => eprintln!("[watcher] event err: {e}"),
            }
        }
    });

    Ok(())
}
