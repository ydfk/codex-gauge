use std::time::Duration;

use slint::{ComponentHandle, PhysicalSize, Timer, WindowSize};

use crate::{
    config::AppConfig, model::CodexUsageSnapshot, presentation, windows, AppTray, DetailWindow,
    MainWidget, SettingsWindow, TopWidget,
};

#[derive(Clone)]
pub(super) struct UiBridge {
    pub(super) main: slint::Weak<MainWidget>,
    pub(super) top: slint::Weak<TopWidget>,
    pub(super) detail: slint::Weak<DetailWindow>,
    pub(super) settings: slint::Weak<SettingsWindow>,
    pub(super) tray: slint::Weak<AppTray>,
}

impl UiBridge {
    pub(super) fn apply_snapshot(&self, snapshot: &CodexUsageSnapshot) {
        let data = presentation::gauge_data(snapshot);
        if let Some(main) = self.main.upgrade() {
            main.set_data(data.clone());
        }
        if let Some(top) = self.top.upgrade() {
            top.set_data(data.clone());
            let width = if data.five_visible { 220.0 } else { 170.0 };
            set_logical_size(top.window(), width, 28.0);
        }
        if let Some(detail) = self.detail.upgrade() {
            detail.set_data(data.clone());
            detail.set_version_text(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        }
        if let Some(tray) = self.tray.upgrade() {
            tray.set_status_tooltip(presentation::tray_tooltip(snapshot).into());
        }
    }

    pub(super) fn sync_config(&self, config: &AppConfig) {
        if let Some(main) = self.main.upgrade() {
            main.set_pinned(config.main_always_on_top);
            main.set_locked(config.main_lock_position);
            main.set_panel_opacity(config.opacity);
            style_later(main.as_weak(), config.main_always_on_top, 430.0, 104.0);
        }
        if let Some(top) = self.top.upgrade() {
            top.set_pinned(config.top_always_on_top);
            top.set_locked(config.top_lock_position);
            top.set_panel_opacity(config.opacity);
            let width = if top.get_data().five_visible {
                220.0
            } else {
                170.0
            };
            style_later(top.as_weak(), config.top_always_on_top, width, 28.0);
        }
        if let Some(tray) = self.tray.upgrade() {
            tray.set_main_pinned(config.main_always_on_top);
            tray.set_top_pinned(config.top_always_on_top);
            tray.set_main_locked(config.main_lock_position);
            tray.set_top_locked(config.top_lock_position);
            tray.set_version_text(env!("CARGO_PKG_VERSION").into());
        }
        self.sync_visibility();
    }

    pub(super) fn sync_visibility(&self) {
        if let Some(tray) = self.tray.upgrade() {
            tray.set_main_visible(
                self.main
                    .upgrade()
                    .is_some_and(|window| window.window().is_visible()),
            );
            tray.set_top_visible(
                self.top
                    .upgrade()
                    .is_some_and(|window| window.window().is_visible()),
            );
        }
    }

    pub(super) fn show_main(&self) {
        if let Some(main) = self.main.upgrade() {
            set_logical_size(main.window(), 430.0, 104.0);
            let _ = main.show();
            style_later(main.as_weak(), main.get_pinned(), 430.0, 104.0);
        }
        self.sync_visibility();
    }

    pub(super) fn show_top(&self) {
        if let Some(top) = self.top.upgrade() {
            let width = if top.get_data().five_visible {
                220.0
            } else {
                170.0
            };
            set_logical_size(top.window(), width, 28.0);
            let _ = top.show();
            style_later(top.as_weak(), top.get_pinned(), width, 28.0);
        }
        self.sync_visibility();
    }

    pub(super) fn toggle_detail(&self) {
        if let Some(detail) = self.detail.upgrade() {
            if detail.window().is_visible() {
                let _ = detail.hide();
            } else {
                set_logical_size(detail.window(), 460.0, 520.0);
                let _ = detail.show();
                style_later(detail.as_weak(), false, 460.0, 520.0);
                windows::bring_to_front(detail.window());
            }
        }
    }

    pub(super) fn bring_visible_to_front(&self) {
        if let Some(main) = self
            .main
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(main.window());
        }
        if let Some(top) = self
            .top
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(top.window());
        }
        if let Some(detail) = self
            .detail
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(detail.window());
        }
        if let Some(settings) = self
            .settings
            .upgrade()
            .filter(|value| value.window().is_visible())
        {
            windows::bring_to_front(settings.window());
        }
    }

    pub(super) fn set_update_message(&self, message: &str) {
        if let Some(settings) = self.settings.upgrade() {
            settings.set_update_status_text(message.into());
        }
    }

    pub(super) fn set_update_state(&self, message: &str, label: &str, available: bool) {
        self.set_update_message(message);
        if let Some(tray) = self.tray.upgrade() {
            tray.set_update_label(label.into());
            tray.set_update_available(available);
        }
    }
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
    let scale = windows::dpi_scale(window).unwrap_or_else(|| window.scale_factor());
    window.set_size(WindowSize::Physical(PhysicalSize::new(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    )));
}
