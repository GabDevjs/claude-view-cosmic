//! Reads the local machine's usage straight from
//! `~/.claude/claudeview_cache.json`. Tolerates every schema the statusLine
//! hook might write (flat `fiveHourPct`, the raw `rate_limits` payload, or the
//! nested `subscription.*` shape) — same normalization the Python collector does.

use serde_json::Value;

#[derive(Default, Clone, Copy)]
pub struct Quota {
    /// 5-hour window used %, 0-100.
    pub five: Option<f64>,
    /// Unix timestamp (seconds) when the 5-hour window resets.
    pub five_reset: Option<i64>,
}

pub fn read() -> Quota {
    let Some(home) = dirs::home_dir() else {
        return Quota::default();
    };
    let path = home.join(".claude").join("claudeview_cache.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Quota::default();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return Quota::default();
    };
    normalize(&v)
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn normalize(v: &Value) -> Quota {
    // 1) Flat ClaudeView cache: {"fiveHourPct": .., "fiveHourResetsAt": ..}
    if let Some(five) = v.get("fiveHourPct").and_then(num) {
        return Quota {
            five: Some(five),
            five_reset: v.get("fiveHourResetsAt").and_then(num).map(|f| f as i64),
        };
    }

    // 2) Raw statusLine payload: rate_limits.five_hour.{used_percentage,resets_at}
    if let Some(rl) = v.get("rate_limits").and_then(|r| r.get("five_hour")) {
        let five = rl.get("used_percentage").and_then(num);
        if five.is_some() {
            return Quota {
                five,
                five_reset: rl.get("resets_at").and_then(num).map(|f| f as i64),
            };
        }
    }

    // 3) Nested subscription shape: subscription.fiveHourLimit.used (total 100)
    if let Some(sub) = v.get("subscription").and_then(|s| s.get("fiveHourLimit")) {
        let five = sub.get("used").and_then(num);
        if five.is_some() {
            return Quota {
                five,
                five_reset: sub.get("resetsAt").and_then(num).map(|f| f as i64),
            };
        }
    }

    Quota::default()
}
