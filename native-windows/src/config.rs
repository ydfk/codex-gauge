use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub start_on_boot: bool,
    pub show_main_on_startup: bool,
    pub show_top_on_startup: bool,
    pub main_always_on_top: bool,
    pub top_always_on_top: bool,
    pub main_lock_position: bool,
    pub top_lock_position: bool,
    pub oled_shift_enabled: bool,
    pub opacity: f32,
    pub refresh_interval_seconds: u64,
    pub preferred_provider: ProviderPreference,
    pub codex_command: String,
    pub update: UpdateConfig,
    pub windows: WindowPositions,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderPreference {
    AppServer,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    pub check_on_startup: bool,
    pub endpoint: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowPositions {
    pub main_x: Option<i32>,
    pub main_y: Option<i32>,
    pub top_x: Option<i32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            start_on_boot: false,
            show_main_on_startup: true,
            show_top_on_startup: true,
            main_always_on_top: false,
            top_always_on_top: true,
            main_lock_position: false,
            top_lock_position: false,
            oled_shift_enabled: false,
            opacity: 0.92,
            refresh_interval_seconds: 60,
            preferred_provider: ProviderPreference::AppServer,
            codex_command: "codex".to_string(),
            update: UpdateConfig {
                check_on_startup: true,
                endpoint: "https://github.com/ydfk/codex-gauge/releases/download/native-latest/latest-native-windows.json"
                    .to_string(),
                public_key: option_env!("CODEX_GAUGE_UPDATER_PUBKEY")
                    .unwrap_or_default()
                    .to_string(),
            },
            windows: WindowPositions::default(),
        }
    }
}

impl AppConfig {
    pub fn normalize(&mut self) {
        self.refresh_interval_seconds = self.refresh_interval_seconds.clamp(30, 3600);
        self.opacity = self.opacity.clamp(0.68, 1.0);
        if self.codex_command.trim().is_empty() {
            self.codex_command = "codex".to_string();
        }
        if self.update.endpoint.trim().is_empty() {
            self.update.endpoint = AppConfig::default().update.endpoint;
        }
        if self.update.public_key.trim().is_empty() {
            self.update.public_key = option_env!("CODEX_GAUGE_UPDATER_PUBKEY")
                .unwrap_or_default()
                .to_string();
        }
    }
}
