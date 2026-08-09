//! Runs `collect_status.py` — the single source of truth for a machine's
//! Claude Code status — either locally (`python3 - …`) or on a remote host
//! (`ssh <alias> python3 - …`), and decodes its JSON stdout.
//!
//! The script is embedded in the binary and piped over stdin in *both* cases,
//! so the local and remote paths are identical and a remote host needs nothing
//! installed but `python3` + key-based SSH. This mirrors the macOS ClaudeView
//! Collector, so a single Python implementation serves every platform.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

/// The collector travels over the wire — embed it so there is never an external
/// file to locate or ship separately.
const COLLECTOR_PY: &str = include_str!("../../scripts/collect_status.py");

/// One machine ClaudeView polls: the local box (`ssh_host == None`) or a remote
/// reachable through a `~/.ssh/config` alias (e.g. "pop").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub label: String,
    pub ssh_host: Option<String>,
}

impl SourceConfig {
    pub fn local() -> Self {
        Self {
            label: "Local".into(),
            ssh_host: None,
        }
    }

    pub fn remote(alias: &str) -> Self {
        Self {
            label: alias.into(),
            ssh_host: Some(alias.into()),
        }
    }

    /// Stable identity used to reconcile state across refreshes.
    pub fn key(&self) -> &str {
        self.ssh_host.as_deref().unwrap_or("__local__")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quota {
    pub model: Option<String>,
    pub model_display: Option<String>,
    pub five_hour_pct: Option<f64>,
    pub five_hour_resets_at: Option<f64>,
    pub seven_day_pct: Option<f64>,
    pub seven_day_resets_at: Option<f64>,
    pub context_window_pct: Option<f64>,
    pub updated_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSession {
    pub pid: u32,
    pub cwd: Option<String>,
    pub project: Option<String>,
    pub running_for: Option<String>,
    pub session_id: Option<String>,
    pub last_modified: Option<f64>,
    pub context_window_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    #[serde(default)]
    pub messages: u64,
    #[serde(default)]
    pub tool_calls: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub window_days: u32,
    pub total_messages: u64,
    pub total_tool_calls: u64,
    /// Keyed by model id. BTreeMap keeps a stable order for the UI.
    pub by_model: BTreeMap<String, ModelStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub hostname: String,
    pub generated_at: f64,
    pub quota: Option<Quota>,
    pub active_sessions: Vec<ActiveSession>,
    pub token_stats: TokenStats,
}

/// Run the collector for one source and decode its JSON.
///
/// `timeout` bounds the whole call via coreutils `timeout(1)` (present on every
/// target Linux); for remotes, SSH's own `ConnectTimeout` bounds a dead host
/// faster than that.
pub fn fetch(ssh_host: Option<&str>, window_days: u32, timeout: Duration) -> Result<SourceSnapshot> {
    let secs = timeout.as_secs().max(1).to_string();
    let window = window_days.max(1).to_string();

    let mut cmd = Command::new("timeout");
    cmd.arg(&secs);
    match ssh_host {
        Some(host) => {
            cmd.args([
                "ssh",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=6",
                host,
                "python3",
                "-",
                "--window-days",
                &window,
            ]);
        }
        None => {
            cmd.args(["python3", "-", "--window-days", &window]);
        }
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawning collector (host={ssh_host:?})"))?;

    // Pipe the script to `python3 -` on stdin, then close so the interpreter
    // sees EOF and starts running. The script is a few KB — well under the pipe
    // buffer — so this write never blocks against unread stdout.
    child
        .stdin
        .take()
        .context("collector stdin unavailable")?
        .write_all(COLLECTOR_PY.as_bytes())
        .context("writing collector script to stdin")?;

    let output = child
        .wait_with_output()
        .context("waiting for collector output")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        // `timeout` exits 124 when it had to kill the child.
        if output.status.code() == Some(124) {
            bail!("timed out after {secs}s");
        }
        let detail = if stderr.is_empty() {
            format!("exit {:?}", output.status.code())
        } else {
            stderr.to_string()
        };
        bail!("{detail}");
    }

    serde_json::from_slice(&output.stdout)
        .context("decoding collector JSON")
        .map_err(|e| {
            let out = String::from_utf8_lossy(&output.stdout);
            anyhow::anyhow!("{e}: {}", out.chars().take(200).collect::<String>())
        })
}

/// The 5-hour usage % from a snapshot, if present. The tray/notifier track this
/// specific window (not context %, which merely resets per session) because it
/// is the one that actually throttles usage.
pub fn five_hour_pct(snapshot: &SourceSnapshot) -> Option<f64> {
    snapshot.quota.as_ref()?.five_hour_pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_collector_shape() {
        let json = r#"{
            "hostname": "box",
            "generatedAt": 1785014330.1,
            "quota": {
                "model": "claude-opus-4-8",
                "modelDisplay": "Opus 4.8",
                "fiveHourPct": 34,
                "fiveHourResetsAt": 1785017400,
                "sevenDayPct": 55.0,
                "sevenDayResetsAt": 1785322800,
                "contextWindowPct": 13,
                "updatedAt": 1785014329.0
            },
            "activeSessions": [
                {"pid": 188365, "cwd": "/home/x/proj", "project": "proj",
                 "runningFor": "16:48", "sessionId": "abc",
                 "lastModified": 1785014169.0, "contextWindowPct": 100.0}
            ],
            "tokenStats": {
                "windowDays": 7, "totalMessages": 3, "totalToolCalls": 5,
                "byModel": {"claude-opus-4-8": {"messages": 3, "toolCalls": 5,
                    "inputTokens": 100, "outputTokens": 50,
                    "cacheReadTokens": 0, "cacheCreationTokens": 0}}
            }
        }"#;
        let snap: SourceSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.hostname, "box");
        assert_eq!(five_hour_pct(&snap), Some(34.0));
        assert_eq!(snap.active_sessions.len(), 1);
        assert_eq!(snap.active_sessions[0].project.as_deref(), Some("proj"));
        assert_eq!(snap.token_stats.by_model["claude-opus-4-8"].tool_calls, 5);
    }

    #[test]
    fn source_config_keys() {
        assert_eq!(SourceConfig::local().key(), "__local__");
        assert_eq!(SourceConfig::remote("pop").key(), "pop");
        assert_eq!(SourceConfig::remote("pop").ssh_host.as_deref(), Some("pop"));
    }
}
