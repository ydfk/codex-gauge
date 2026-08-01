use serde::{Deserialize, Serialize};

const DEFAULT_UPDATE_ENDPOINT: &str =
    "https://github.com/ydfk/codex-gauge/releases/latest/download/latest.json";
const LEGACY_UPDATE_ENDPOINT: &str = "https://github.com/ydfk/codex-gauge/releases/download/native-latest/latest-native-windows.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub start_on_boot: bool,
    pub show_top_on_startup: bool,
    pub top_bar_display: TopBarDisplay,
    pub top_always_on_top: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TopBarDisplay {
    FiveAndSeven,
    FiveHour,
    IconOnly,
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
    pub top_x: Option<i32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 3,
            start_on_boot: false,
            show_top_on_startup: true,
            top_bar_display: TopBarDisplay::FiveHour,
            top_always_on_top: true,
            top_lock_position: false,
            oled_shift_enabled: false,
            opacity: 0.92,
            refresh_interval_seconds: 60,
            preferred_provider: ProviderPreference::AppServer,
            codex_command: "codex".to_string(),
            update: UpdateConfig {
                check_on_startup: true,
                endpoint: DEFAULT_UPDATE_ENDPOINT.to_string(),
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
        self.version = 3;
        self.refresh_interval_seconds = self.refresh_interval_seconds.clamp(30, 3600);
        self.opacity = self.opacity.clamp(0.68, 1.0);
        if self.codex_command.trim().is_empty() {
            self.codex_command = "codex".to_string();
        }
        if self.update.endpoint.trim().is_empty() || self.update.endpoint == LEGACY_UPDATE_ENDPOINT
        {
            self.update.endpoint = DEFAULT_UPDATE_ENDPOINT.to_string();
        }
        if self.update.public_key.trim().is_empty() {
            self.update.public_key = option_env!("CODEX_GAUGE_UPDATER_PUBKEY")
                .unwrap_or_default()
                .to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_uses_compact_default_display_mode() {
        let mut config: AppConfig = serde_json::from_str(
            r#"{
                "version": 2,
                "refreshIntervalSeconds": 60,
                "preferredProvider": "app-server",
                "codexCommand": "codex",
                "update": {
                    "checkOnStartup": true,
                    "endpoint": "https://example.com/latest.json",
                    "publicKey": ""
                },
                "windows": {}
            }"#,
        )
        .unwrap();

        config.normalize();

        assert_eq!(config.version, 3);
        assert_eq!(config.top_bar_display, TopBarDisplay::FiveHour);
    }

    #[test]
    fn serializes_all_top_bar_display_modes() {
        for (mode, expected) in [
            (TopBarDisplay::FiveAndSeven, "\"five-and-seven\""),
            (TopBarDisplay::FiveHour, "\"five-hour\""),
            (TopBarDisplay::IconOnly, "\"icon-only\""),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), expected);
        }
    }

    #[test]
    fn migrates_legacy_update_endpoint() {
        let mut config = AppConfig::default();
        config.update.endpoint = LEGACY_UPDATE_ENDPOINT.to_string();

        config.normalize();

        assert_eq!(config.update.endpoint, DEFAULT_UPDATE_ENDPOINT);
    }
}
