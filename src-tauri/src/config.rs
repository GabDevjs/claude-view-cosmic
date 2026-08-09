use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::collector::SourceConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct DisplayPrefs {
    pub show_percentage: bool,
    pub alert_threshold: u32,
    pub history_window_days: u32,
    pub refresh_interval_seconds: u32,
}

impl Default for DisplayPrefs {
    fn default() -> Self {
        // Literal defaults from PRA §4.5.
        Self {
            show_percentage: true,
            alert_threshold: 80,
            history_window_days: 7,
            refresh_interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct BehaviorPrefs {
    pub autostart: bool,
    pub notifications_enabled: bool,
    pub notification_thresholds: Vec<u32>,
}

impl Default for BehaviorPrefs {
    fn default() -> Self {
        Self {
            // PRA §4.5 says autostart=true, but we keep it false here so the
            // first launch does not silently write a .desktop file before
            // the user has opened Preferences. Flipping the toggle in the UI
            // (or running `make install`) is what enables it.
            autostart: false,
            notifications_enabled: true,
            notification_thresholds: vec![80, 95],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct AppearancePrefs {
    pub theme: String,
}

impl Default for AppearancePrefs {
    fn default() -> Self {
        Self {
            theme: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Preferences {
    /// `~/.ssh/config` aliases of remote machines to poll alongside the local
    /// one (e.g. `["pop"]`). Each must be reachable via passwordless `ssh <alias>`.
    ///
    /// Declared first so TOML serializes it as a top-level array *before* the
    /// section tables — `toml` rejects a bare value that follows a `[table]`.
    pub remote_hosts: Vec<String>,
    pub display: DisplayPrefs,
    pub behavior: BehaviorPrefs,
    pub appearance: AppearancePrefs,
}

impl Preferences {
    /// The full source list the poller iterates: local first, then each
    /// non-empty remote alias.
    pub fn sources(&self) -> Vec<SourceConfig> {
        let mut sources = vec![SourceConfig::local()];
        for host in &self.remote_hosts {
            let host = host.trim();
            if !host.is_empty() {
                sources.push(SourceConfig::remote(host));
            }
        }
        sources
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("$XDG_CONFIG_HOME / $HOME is not set")?;
    Ok(base.join("claudeview").join("config.toml"))
}

/// Read prefs from disk. Missing file → defaults. Malformed file → error
/// (caller decides whether to surface or fall back).
pub fn load() -> Result<Preferences> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let prefs: Preferences = toml::from_str(&text)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(prefs)
}

/// Persist prefs atomically: write to a .tmp sibling and rename, so the
/// config file is never half-written even if the process dies mid-save.
pub fn save(prefs: &Preferences) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(prefs).context("serializing prefs to toml")?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_pra() {
        let p = Preferences::default();
        assert!(p.display.show_percentage);
        assert_eq!(p.display.alert_threshold, 80);
        assert_eq!(p.display.history_window_days, 7);
        assert_eq!(p.display.refresh_interval_seconds, 60);
        // Behavior.autostart intentionally false (see comment in impl).
        assert!(!p.behavior.autostart);
        assert!(p.behavior.notifications_enabled);
        assert_eq!(p.behavior.notification_thresholds, vec![80, 95]);
        assert_eq!(p.appearance.theme, "auto");
    }

    #[test]
    fn round_trips_through_toml() {
        let mut p = Preferences::default();
        p.display.alert_threshold = 90;
        p.behavior.autostart = true;
        p.appearance.theme = "dark".into();
        let s = toml::to_string_pretty(&p).unwrap();
        let back: Preferences = toml::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn partial_file_keeps_defaults_for_missing_sections() {
        let s = r#"
            [display]
            alert_threshold = 90
        "#;
        let p: Preferences = toml::from_str(s).unwrap();
        assert_eq!(p.display.alert_threshold, 90);
        assert_eq!(p.display.history_window_days, 7); // default kept
        assert_eq!(p.appearance.theme, "auto"); // section absent → default
    }
}
