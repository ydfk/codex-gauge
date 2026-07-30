use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Local;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{config::AppConfig, model::CodexUsageSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StateDocument {
    pub last_snapshot: Option<CodexUsageSnapshot>,
    pub consecutive_failures: u32,
    pub last_success_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AppStorage {
    root: PathBuf,
}

impl AppStorage {
    pub fn new() -> Self {
        let root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("CodexGaugeNative");
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    pub fn load_config(&self) -> AppConfig {
        let mut config: AppConfig = read_json(&self.root.join("config.json")).unwrap_or_default();
        config.normalize();
        self.save_config(&config);
        config
    }

    pub fn save_config(&self, config: &AppConfig) {
        write_json(&self.root.join("config.json"), config);
    }

    pub fn load_state(&self) -> StateDocument {
        read_json(&self.root.join("state.json")).unwrap_or_default()
    }

    pub fn save_state(&self, state: &StateDocument) {
        write_json(&self.root.join("state.json"), state);
    }

    pub fn record_update_event(&self, method: &str, outcome: &str, category: &str) {
        let path = self.root.join("update.log");
        if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 256 * 1024) {
            let _ = fs::write(&path, "");
        }
        let timestamp = Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
        let line = format!("{timestamp} method={method} outcome={outcome} category={category}\n");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    let Ok(content) = serde_json::to_string_pretty(value) else {
        return;
    };
    let temp = path.with_extension("tmp");
    if fs::write(&temp, content).is_ok() {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        if fs::rename(&temp, path).is_err() {
            let _ = fs::copy(&temp, path);
            let _ = fs::remove_file(temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_isolated_default_config() {
        let temp = tempfile::tempdir().expect("temp dir");
        let storage = AppStorage::with_root(temp.path().to_path_buf());
        let config = storage.load_config();

        assert!(config.top_always_on_top);
        assert!(temp.path().join("config.json").exists());
    }

    #[test]
    fn overwrites_existing_config_on_windows() {
        let temp = tempfile::tempdir().expect("temp dir");
        let storage = AppStorage::with_root(temp.path().to_path_buf());
        let mut config = storage.load_config();
        config.top_lock_position = true;
        config.windows.top_x = Some(320);
        storage.save_config(&config);

        let loaded = storage.load_config();
        assert!(loaded.top_lock_position);
        assert_eq!(loaded.windows.top_x, Some(320));
    }

    #[test]
    fn update_log_never_contains_payloads() {
        let temp = tempfile::tempdir().expect("temp dir");
        let storage = AppStorage::with_root(temp.path().to_path_buf());
        storage.record_update_event("check", "failed", "network");

        let log = fs::read_to_string(temp.path().join("update.log")).expect("update log");
        assert!(log.contains("category=network"));
        assert!(!log.contains("Authorization"));
    }
}
