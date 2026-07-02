use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub label: String,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
    pub reset_remaining_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub today_tokens: Option<i64>,
    pub week_tokens: Option<i64>,
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    Ok,
    NotLoggedIn,
    CodexNotFound,
    AppServerError,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexGaugeSnapshot {
    pub account_email: Option<String>,
    pub plan_type: Option<String>,
    pub five_hour: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub other_windows: Vec<UsageWindow>,
    pub reset: super::ResetStats,
    pub token_usage: TokenUsage,
    pub credits: Option<Value>,
    pub rate_limit_reached_type: Option<String>,
    pub last_updated_at: i64,
    pub source: String,
    pub status: SnapshotStatus,
}

pub fn empty_snapshot() -> CodexGaugeSnapshot {
    CodexGaugeSnapshot {
        account_email: None,
        plan_type: None,
        five_hour: None,
        weekly: None,
        other_windows: Vec::new(),
        reset: super::ResetStats::default(),
        token_usage: TokenUsage::default(),
        credits: None,
        rate_limit_reached_type: None,
        last_updated_at: chrono::Local::now().timestamp(),
        source: "codex-app-server".to_string(),
        status: SnapshotStatus::AppServerError,
    }
}
