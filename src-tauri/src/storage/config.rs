use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub general: GeneralConfig,
    pub codex: CodexConfig,
    pub update: UpdateConfig,
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    pub start_on_boot: bool,
    pub show_on_startup: bool,
    pub always_on_top: bool,
    pub lock_position: bool,
    pub opacity: f32,
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexConfig {
    pub command: String,
    pub transport: String,
    pub enable_usage_read: bool,
    pub enable_reset_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    pub auto_check: bool,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowConfig {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: f64,
    pub height: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            general: GeneralConfig {
                start_on_boot: false,
                show_on_startup: true,
                always_on_top: true,
                lock_position: false,
                opacity: 0.92,
                refresh_interval_seconds: 60,
            },
            codex: CodexConfig {
                command: "codex".to_string(),
                transport: "stdio".to_string(),
                enable_usage_read: true,
                enable_reset_stats: true,
            },
            update: UpdateConfig {
                auto_check: true,
                channel: "stable".to_string(),
            },
            window: WindowConfig {
                x: None,
                y: None,
                width: 360.0,
                height: 142.0,
            },
        }
    }
}
