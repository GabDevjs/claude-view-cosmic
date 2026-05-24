use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    pub model: String,
    pub messages: u64,
    pub tool_calls: u64,
    pub tokens: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub by_model: Vec<ModelStats>,
    pub total_messages: u64,
    pub total_tool_calls: u64,
    pub total_tokens: u64,
    /// Unix seconds — cutoff used for filtering. Frontend can show "since X".
    pub since: i64,
    pub days: u32,
}

pub fn projects_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("$HOME is not set")?;
    Ok(home.join(".claude").join("projects"))
}

pub fn aggregate(days: u32) -> Result<TokenStats> {
    let cutoff = Utc::now() - Duration::days(days.max(1) as i64);
    let root = projects_dir()?;

    let mut totals: BTreeMap<String, ModelStats> = BTreeMap::new();

    if !root.is_dir() {
        // No projects directory yet — return empty stats rather than erroring.
        return Ok(TokenStats {
            since: cutoff.timestamp(),
            days,
            ..Default::default()
        });
    }

    for project_entry in std::fs::read_dir(&root)
        .with_context(|| format!("reading {}", root.display()))?
    {
        let Ok(project_entry) = project_entry else { continue };
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&project_path) else { continue };
        for file_entry in files.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            // Errors on a single file should not abort the whole aggregate.
            if let Err(e) = aggregate_file(&path, cutoff, &mut totals) {
                eprintln!("[transcripts] skip {}: {e}", path.display());
            }
        }
    }

    let by_model: Vec<ModelStats> = totals.into_values().collect();
    let total_messages = by_model.iter().map(|m| m.messages).sum();
    let total_tool_calls = by_model.iter().map(|m| m.tool_calls).sum();
    let total_tokens = by_model.iter().map(|m| m.tokens).sum();

    Ok(TokenStats {
        by_model,
        total_messages,
        total_tool_calls,
        total_tokens,
        since: cutoff.timestamp(),
        days,
    })
}

fn aggregate_file(
    path: &Path,
    cutoff: DateTime<Utc>,
    totals: &mut BTreeMap<String, ModelStats>,
) -> Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        // Date filter: skip entries older than the cutoff. Entries without
        // a parseable timestamp are kept (treated as "recent enough") so a
        // weird transcript does not silently drop data.
        if let Some(ts) = value.get("timestamp").and_then(Value::as_str) {
            if let Ok(ts) = DateTime::parse_from_rfc3339(ts) {
                if ts.with_timezone(&Utc) < cutoff {
                    continue;
                }
            }
        }
        accumulate(&value, totals);
    }
    Ok(())
}

fn accumulate(value: &Value, totals: &mut BTreeMap<String, ModelStats>) {
    let Some(message) = value.get("message") else { return };
    let model = message
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    let entry = totals.entry(model.clone()).or_insert_with(|| ModelStats {
        model,
        ..Default::default()
    });

    entry.messages += 1;
    entry.tokens += tokens_of(message);
    entry.tool_calls += tool_uses_in(message);
}

fn tokens_of(message: &Value) -> u64 {
    let Some(usage) = message.get("usage") else { return 0 };
    const KEYS: &[&str] = &[
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ];
    KEYS.iter()
        .map(|k| usage.get(*k).and_then(Value::as_u64).unwrap_or(0))
        .sum()
}

fn tool_uses_in(message: &Value) -> u64 {
    let Some(content) = message.get("content").and_then(Value::as_array) else {
        return 0;
    };
    content
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_assistant_line(model: &str, in_tok: u64, out_tok: u64, tool_uses: usize) -> String {
        let content: Vec<Value> = (0..tool_uses)
            .map(|i| serde_json::json!({"type": "tool_use", "name": format!("tool_{i}")}))
            .collect();
        serde_json::json!({
            "type": "assistant",
            "timestamp": Utc::now().to_rfc3339(),
            "message": {
                "model": model,
                "usage": {
                    "input_tokens": in_tok,
                    "output_tokens": out_tok,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                },
                "content": content
            }
        })
        .to_string()
    }

    #[test]
    fn aggregates_tokens_messages_and_tool_calls() {
        let tmp = tempdir();
        let project = tmp.join("-fake-proj");
        std::fs::create_dir_all(&project).unwrap();
        let jsonl = project.join("sess.jsonl");
        let mut content = String::new();
        content.push_str(&synthetic_assistant_line("claude-opus-4-7", 100, 50, 2));
        content.push('\n');
        content.push_str(&synthetic_assistant_line("claude-opus-4-7", 200, 80, 0));
        content.push('\n');
        content.push_str(&synthetic_assistant_line("claude-sonnet-4-6", 30, 10, 1));
        content.push('\n');
        std::fs::write(&jsonl, content).unwrap();

        let mut totals = BTreeMap::new();
        aggregate_file(&jsonl, Utc::now() - Duration::days(1), &mut totals).unwrap();

        let opus = totals.get("claude-opus-4-7").unwrap();
        assert_eq!(opus.messages, 2);
        assert_eq!(opus.tool_calls, 2);
        assert_eq!(opus.tokens, 100 + 50 + 200 + 80);

        let sonnet = totals.get("claude-sonnet-4-6").unwrap();
        assert_eq!(sonnet.messages, 1);
        assert_eq!(sonnet.tool_calls, 1);
        assert_eq!(sonnet.tokens, 40);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn ignores_non_assistant_lines_and_old_timestamps() {
        let tmp = tempdir();
        let project = tmp.join("-fake-proj-2");
        std::fs::create_dir_all(&project).unwrap();
        let jsonl = project.join("sess.jsonl");
        let mut content = String::new();
        // user line — must be ignored
        content.push_str(r#"{"type":"user","message":{"content":"hi"}}"#);
        content.push('\n');
        // old assistant line — must be filtered out
        content.push_str(&serde_json::json!({
            "type": "assistant",
            "timestamp": "2020-01-01T00:00:00Z",
            "message": {"model": "old", "usage": {"input_tokens": 999, "output_tokens": 0}, "content": []}
        }).to_string());
        content.push('\n');
        // fresh assistant — included
        content.push_str(&synthetic_assistant_line("fresh-model", 5, 5, 0));
        content.push('\n');
        std::fs::write(&jsonl, content).unwrap();

        let mut totals = BTreeMap::new();
        aggregate_file(&jsonl, Utc::now() - Duration::days(7), &mut totals).unwrap();

        assert!(!totals.contains_key("old"));
        assert_eq!(totals.get("fresh-model").unwrap().tokens, 10);

        std::fs::remove_dir_all(&tmp).ok();
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("claudeview_test_{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
