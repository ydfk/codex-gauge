use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub name: String,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<i64>,
    pub window_duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageCredits {
    pub available_count: Option<i64>,
    pub reset_at: Option<i64>,
    pub items: Vec<ResetCreditItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditItem {
    pub status: Option<String>,
    pub title: Option<String>,
    pub granted_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotSource {
    AuthJson,
    AppServer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    Ok,
    NotLoggedIn,
    InvalidAuth,
    RequestFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageSnapshot {
    pub source: SnapshotSource,
    pub status: SnapshotStatus,
    pub plan_type: Option<String>,
    pub primary_window: Option<UsageWindow>,
    #[serde(default)]
    pub primary_window_unlimited: bool,
    pub secondary_window: Option<UsageWindow>,
    pub credits: Option<UsageCredits>,
    pub rate_limit_reached_type: Option<String>,
    pub updated_at: i64,
}

impl CodexUsageSnapshot {
    pub fn empty(source: SnapshotSource, status: SnapshotStatus) -> Self {
        Self {
            source,
            status,
            plan_type: None,
            primary_window: None,
            primary_window_unlimited: false,
            secondary_window: None,
            credits: None,
            rate_limit_reached_type: None,
            updated_at: chrono::Local::now().timestamp(),
        }
    }

    pub fn normalize(&mut self) {
        self.updated_at = chrono::Local::now().timestamp();
        self.primary_window_unlimited = self.status == SnapshotStatus::Ok
            && self.primary_window.is_none()
            && self.secondary_window.is_some();
    }
}
