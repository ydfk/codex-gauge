use std::time::Duration;

use slint::{ComponentHandle, LogicalSize, Timer, WindowSize};

use crate::{
    config::{AppConfig, TopBarDisplay},
    model::CodexUsageSnapshot,
    presentation, windows, AppTray, PanelPage, PanelWindow, TopWidget,
};

pub(super) const TOP_HEIGHT: f32 = 22.0;
pub(super) const PANEL_WIDTH: f32 = 376.0;
pub(super) const PANEL_HEIGHT: f32 = 510.0;

#[derive(Clone)]
pub(super) struct UiBridge {
    pub(super) top: slint::Weak<TopWidget>,
    pub(super) panel: slint::Weak<PanelWindow>,
    pub(super) tray: slint::Weak<AppTray>,
}

impl UiBridge {
    pub(super) fn apply_snapshot(&self, snapshot: &CodexUsageSnapshot) {
        let data = presentation::gauge_data(snapshot);
        if let Some(top) = self.top.upgrade() {
            top.set_data(data.clone());
            let width = top_width(top.get_display_mode().as_str(), data.five_visible);
            set_logical_size(top.window(), width, TOP_HEIGHT);
        }
        if let Some(panel) = self.panel.upgrade() {
            panel.set_data(data);
            panel.set_version_text(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        }
        if let Some(tray) = self.tray.upgrade() {
            tray.set_status_tooltip(presentation::tray_tooltip(snapshot).into());
        }
    }

    pub(super) fn sync_config(&self, config: &AppConfig) {
        let display_mode = display_mode(config.top_bar_display);
        if let Some(top) = self.top.upgrade() {
            top.set_display_mode(display_mode.into());
            top.set_pinned(config.top_always_on_top);
            top.set_locked(config.top_lock_position);
            top.set_panel_opacity(config.opacity);
            style_top_later(top.as_weak());
        }
        if let Some(panel) = self.panel.upgrade() {
            panel.set_start_on_boot(config.start_on_boot);
            panel.set_show_top(config.show_top_on_startup);
            panel.set_top_pinned(config.top_always_on_top);
            panel.set_top_locked(config.top_lock_position);
            panel.set_oled_shift(config.oled_shift_enabled);
            panel.set_panel_opacity(config.opacity * 100.0);
            panel.set_display_mode(display_mode.into());
            panel.set_refresh_seconds(config.refresh_interval_seconds.to_string().into());
            panel.set_codex_command(config.codex_command.clone().into());
            panel.set_provider(
                match config.preferred_provider {
                    crate::config::ProviderPreference::AppServer => "app-server",
                    crate::config::ProviderPreference::Api => "api",
                }
                .into(),
            );
            panel.set_update_on_start(config.update.check_on_startup);
            panel.set_update_endpoint(config.update.endpoint.clone().into());
            panel.set_version_text(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        }
        if let Some(tray) = self.tray.upgrade() {
            tray.set_top_pinned(config.top_always_on_top);
            tray.set_top_locked(config.top_lock_position);
            tray.set_version_text(env!("CARGO_PKG_VERSION").into());
        }
        self.sync_visibility();
    }

    pub(super) fn sync_visibility(&self) {
        if let Some(tray) = self.tray.upgrade() {
            tray.set_top_visible(
                self.top
                    .upgrade()
                    .is_some_and(|window| window.window().is_visible()),
            );
        }
    }

    pub(super) fn show_top(&self) {
        if let Some(top) = self.top.upgrade() {
            let width = top_width(top.get_display_mode().as_str(), top.get_data().five_visible);
            set_logical_size(top.window(), width, TOP_HEIGHT);
            let _ = top.show();
            style_top_later(top.as_weak());
        }
        self.sync_visibility();
    }

    pub(super) fn show_panel(&self, page: PanelPage) {
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        panel.set_page(page);
        set_logical_size(panel.window(), PANEL_WIDTH, PANEL_HEIGHT);
        let _ = panel.show();
        style_later(panel.as_weak(), false, PANEL_WIDTH, PANEL_HEIGHT);

        let panel_weak = panel.as_weak();
        let top_weak = self.top.clone();
        Timer::single_shot(Duration::from_millis(100), move || {
            let Some(panel) = panel_weak.upgrade() else {
                return;
            };
            if let Some(top) = top_weak.upgrade().filter(|top| top.window().is_visible()) {
                windows::place_below(top.window(), panel.window());
            } else {
                let (width, height) = windows::scaled_size(PANEL_WIDTH as i32, PANEL_HEIGHT as i32);
                let (x, y) = windows::default_main_position(width, height);
                windows::set_position(panel.window(), x, y);
            }
            windows::bring_to_front(panel.window());
        });
    }

    pub(super) fn toggle_panel(&self) {
        if let Some(panel) = self.panel.upgrade() {
            if panel.window().is_visible() {
                let _ = panel.hide();
            } else {
                self.show_panel(PanelPage::Usage);
            }
        }
    }

    pub(super) fn bring_visible_to_front(&self) {
        if let Some(top) = self
            .top
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(top.window());
        }
        if let Some(panel) = self
            .panel
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(panel.window());
        }
    }

    pub(super) fn set_update_message(&self, message: &str) {
        if let Some(panel) = self.panel.upgrade() {
            panel.set_update_status_text(message.into());
        }
    }

    pub(super) fn set_login_message(&self, message: &str) {
        if let Some(panel) = self.panel.upgrade() {
            panel.set_login_action_text(message.into());
        }
    }

    pub(super) fn set_update_state(&self, message: &str, label: &str, available: bool) {
        self.set_update_message(message);
        if let Some(panel) = self.panel.upgrade() {
            panel.set_update_action_label(label.into());
            panel.set_update_available(available);
        }
        if let Some(tray) = self.tray.upgrade() {
            tray.set_update_label(label.into());
            tray.set_update_available(available);
        }
    }
}

pub(super) fn top_width(display_mode: &str, five_visible: bool) -> f32 {
    let (show_five, show_seven) = top_metric_visibility(display_mode, five_visible);
    match (show_five, show_seven) {
        (true, true) => 154.0,
        (true, false) | (false, true) => 92.0,
        (false, false) => 34.0,
    }
}

fn top_metric_visibility(display_mode: &str, five_visible: bool) -> (bool, bool) {
    match display_mode {
        "icon-only" => (false, false),
        _ => (five_visible, true),
    }
}

pub(super) fn display_mode(mode: TopBarDisplay) -> &'static str {
    match mode {
        TopBarDisplay::FiveAndSeven | TopBarDisplay::FiveHour => "five-and-seven",
        TopBarDisplay::IconOnly => "icon-only",
    }
}

fn style_top_later(weak: slint::Weak<TopWidget>) {
    Timer::single_shot(Duration::from_millis(80), move || {
        if let Some(top) = weak.upgrade() {
            // 延迟期间额度或显示模式可能改变，不能用旧宽度覆盖最新布局。
            let width = top_width(top.get_display_mode().as_str(), top.get_data().five_visible);
            set_logical_size(top.window(), width, TOP_HEIGHT);
            let _ = windows::apply_native_style(top.window(), top.get_pinned());
        }
    });
}

pub(super) fn style_later<T>(weak: slint::Weak<T>, topmost: bool, width: f32, height: f32)
where
    T: ComponentHandle + 'static,
{
    Timer::single_shot(Duration::from_millis(80), move || {
        if let Some(component) = weak.upgrade() {
            set_logical_size(component.window(), width, height);
            let _ = windows::apply_native_style(component.window(), topmost);
        }
    });
}

pub(super) fn set_logical_size(window: &slint::Window, width: f32, height: f32) {
    window.set_size(WindowSize::Logical(LogicalSize::new(width, height)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_width_matches_display_mode() {
        assert_eq!(top_width("icon-only", true), 34.0);
        assert_eq!(top_width("five-hour", true), 154.0);
        assert_eq!(top_width("five-and-seven", true), 154.0);
    }

    #[test]
    fn unlimited_five_hour_falls_back_to_weekly() {
        assert_eq!(top_metric_visibility("five-hour", false), (false, true));
        assert_eq!(
            top_metric_visibility("five-and-seven", false),
            (false, true)
        );
        assert_eq!(top_width("five-and-seven", false), 92.0);
    }

    #[test]
    fn both_windows_remain_visible_in_legacy_and_default_modes() {
        for mode in ["five-hour", "five-and-seven"] {
            assert_eq!(top_metric_visibility(mode, true), (true, true));
            let widths: Vec<_> = [true, false, true]
                .into_iter()
                .map(|visible| top_width(mode, visible))
                .collect();
            assert_eq!(widths, vec![154.0, 92.0, 154.0]);
        }
        for visible in [true, false] {
            assert_eq!(top_metric_visibility("icon-only", visible), (false, false));
            assert_eq!(top_width("icon-only", visible), 34.0);
        }
    }
}
