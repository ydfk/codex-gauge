use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use super::AppConfig;
use crate::codex::{CodexGaugeSnapshot, ResetStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDocument {
    pub version: u32,
    pub last_snapshot: Option<CodexGaugeSnapshot>,
    pub reset_stats: ResetStats,
    pub stats_start_at: Option<i64>,
    pub last_stats_date: Option<String>,
    pub last_stats_week_start: Option<String>,
}

impl Default for StateDocument {
    fn default() -> Self {
        Self {
            version: 1,
            last_snapshot: None,
            reset_stats: ResetStats::default(),
            stats_start_at: None,
            last_stats_date: None,
            last_stats_week_start: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageHistory {
    version: u32,
    daily: Vec<DailyUsageHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyUsageHistory {
    date: String,
    tokens: Option<i64>,
    snapshots: Vec<UsageHistorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageHistorySnapshot {
    time: String,
    five_hour_used_percent: Option<f64>,
    weekly_used_percent: Option<f64>,
    available_reset_credits: Option<i64>,
}

impl Default for UsageHistory {
    fn default() -> Self {
        Self {
            version: 1,
            daily: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct AppStorage {
    root: PathBuf,
}

impl AppStorage {
    pub fn new() -> Self {
        let root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("CodexGauge");
        let _ = fs::create_dir_all(&root);

        Self { root }
    }

    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    pub fn load_config(&self) -> AppConfig {
        read_json(&self.config_path()).unwrap_or_else(|| {
            let config = AppConfig::default();
            self.save_config(&config);
            config
        })
    }

    pub fn save_config(&self, config: &AppConfig) {
        write_json(&self.config_path(), config);
    }

    pub fn load_state(&self) -> StateDocument {
        read_json(&self.state_path()).unwrap_or_else(|| {
            let state = StateDocument::default();
            self.save_state(&state);
            state
        })
    }

    pub fn save_state(&self, state: &StateDocument) {
        write_json(&self.state_path(), state);
    }

    pub fn record_usage_snapshot(&self, snapshot: &CodexGaugeSnapshot) {
        let today = Local::now().date_naive();
        let mut history: UsageHistory = read_json(&self.usage_history_path()).unwrap_or_default();

        history.daily.retain(|item| {
            NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                .map(|date| today.signed_duration_since(date).num_days() < 90)
                .unwrap_or(false)
        });

        let date = today.format("%Y-%m-%d").to_string();
        let time = Local::now().format("%H:%M:%S").to_string();
        let daily_index = history
            .daily
            .iter()
            .position(|item| item.date == date)
            .unwrap_or_else(|| {
                history.daily.push(DailyUsageHistory {
                    date: date.clone(),
                    tokens: snapshot.token_usage.today_tokens,
                    snapshots: Vec::new(),
                });
                history.daily.len() - 1
            });
        let daily = &mut history.daily[daily_index];

        daily.tokens = snapshot.token_usage.today_tokens;
        daily.snapshots.push(UsageHistorySnapshot {
            time,
            five_hour_used_percent: snapshot
                .five_hour
                .as_ref()
                .and_then(|window| window.used_percent),
            weekly_used_percent: snapshot
                .weekly
                .as_ref()
                .and_then(|window| window.used_percent),
            available_reset_credits: snapshot.reset.available_reset_credits,
        });

        write_json(&self.usage_history_path(), &history);
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn usage_history_path(&self) -> PathBuf {
        self.root.join("usage-history.json")
    }
}

fn read_json<T>(path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_json<T>(path: &Path, value: &T)
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Ok(content) = serde_json::to_string_pretty(value) else {
        return;
    };

    let temp_path = path.with_extension("tmp");
    if fs::write(&temp_path, content).is_ok() {
        let _ = fs::rename(temp_path, path);
    }
}

pub fn current_week_start() -> NaiveDate {
    let today = Local::now().date_naive();
    let offset = today.weekday().num_days_from_monday() as i64;
    today - chrono::Duration::days(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_default_config() {
        let temp = tempfile::tempdir().expect("temp dir");
        let storage = AppStorage::with_root(temp.path().to_path_buf());

        let config = storage.load_config();

        assert_eq!(config.window.width, 360.0);
        assert!(temp.path().join("config.json").exists());
    }
}
