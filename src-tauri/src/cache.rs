use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitData {
    pub used: Option<u64>,
    pub total: Option<u64>,
    pub reset_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionData {
    pub five_hour_limit: Option<LimitData>,
    pub seven_day_limit: Option<LimitData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionData {
    pub model: Option<String>,
    pub context_used: Option<u64>,
    pub context_total: Option<u64>,
    pub messages_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CacheData {
    pub updated_at: Option<f64>,
    pub subscription: Option<SubscriptionData>,
    pub session: Option<SessionData>,
}

pub fn cache_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("$HOME is not set")?;
    Ok(home.join(".claude").join("claudeview_cache.json"))
}

pub fn read_cache() -> Result<CacheData> {
    let path = cache_path()?;
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let data: CacheData = serde_json::from_str(&text)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_target_schema() {
        let json = r#"{
            "updatedAt": 1716553200.123,
            "subscription": {
                "fiveHourLimit": {"used": 45, "total": 100, "resetAt": 1716570000},
                "sevenDayLimit": {"used": 320, "total": 1000, "resetAt": 1717160000}
            },
            "session": {
                "model": "claude-sonnet-4-7",
                "contextUsed": 85000,
                "contextTotal": 200000,
                "messagesCount": 42
            }
        }"#;
        let data: CacheData = serde_json::from_str(json).unwrap();
        assert_eq!(data.updated_at, Some(1716553200.123));
        let sub = data.subscription.unwrap();
        assert_eq!(sub.five_hour_limit.unwrap().used, Some(45));
        assert_eq!(sub.seven_day_limit.unwrap().total, Some(1000));
        let ses = data.session.unwrap();
        assert_eq!(ses.model.as_deref(), Some("claude-sonnet-4-7"));
        assert_eq!(ses.context_used, Some(85000));
    }

    #[test]
    fn parses_post_tool_use_payload() {
        // The current hook writes the raw PostToolUse payload plus updatedAt.
        // Unknown fields must not break parsing.
        let json = r#"{
            "session_id": "abc",
            "tool_name": "Bash",
            "updatedAt": 1779659916.6
        }"#;
        let data: CacheData = serde_json::from_str(json).unwrap();
        assert_eq!(data.updated_at, Some(1779659916.6));
        assert!(data.subscription.is_none());
        assert!(data.session.is_none());
    }
}
