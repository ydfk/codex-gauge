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
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub main_always_on_top: bool,
    #[serde(default)]
    pub top_always_on_top: bool,
    pub lock_position: bool,
    #[serde(default)]
    pub oled_shift_enabled: bool,
    #[serde(default = "default_true")]
    pub top_status_enabled: bool,
    pub opacity: f32,
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexConfig {
    pub command: String,
    pub transport: String,
    #[serde(default = "default_provider")]
    pub preferred_provider: String,
    pub enable_usage_read: bool,
    pub enable_reset_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    pub auto_check: bool,
    pub channel: String,
    #[serde(default = "default_update_endpoint")]
    pub endpoint: String,
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
            version: 5,
            general: GeneralConfig {
                start_on_boot: false,
                show_on_startup: true,
                always_on_top: false,
                main_always_on_top: false,
                top_always_on_top: true,
                lock_position: false,
                oled_shift_enabled: false,
                top_status_enabled: true,
                opacity: 0.92,
                refresh_interval_seconds: 60,
            },
            codex: CodexConfig {
                command: "codex".to_string(),
                transport: "stdio".to_string(),
                preferred_provider: default_provider(),
                enable_usage_read: true,
                enable_reset_stats: true,
            },
            update: UpdateConfig {
                auto_check: true,
                channel: "stable".to_string(),
                endpoint: default_update_endpoint(),
            },
            window: WindowConfig {
                x: None,
                y: None,
                width: 430.0,
                height: 104.0,
            },
        }
    }
}

impl AppConfig {
    pub fn migrate(&mut self) {
        if self.version < 2 {
            self.general.always_on_top = false;
            self.codex.preferred_provider = default_provider();
            self.version = 2;
        }
        if self.version < 3 {
            if self.update.endpoint.trim().is_empty() {
                self.update.endpoint = default_update_endpoint();
            }
            self.version = 3;
        }
        if self.version < 4 {
            self.general.main_always_on_top = self.general.always_on_top;
            self.general.top_always_on_top = self.general.always_on_top;
            self.version = 4;
        }
        if self.version < 5 {
            self.general.top_always_on_top = true;
            self.version = 5;
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_provider() -> String {
    "app-server".to_string()
}

pub fn default_update_endpoint() -> String {
    "https://github.com/ydfk/codex-gauge/releases/latest/download/latest.json".to_string()
}
