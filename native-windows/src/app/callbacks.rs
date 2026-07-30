use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use slint::ComponentHandle;

use crate::{windows, DetailWindow, SettingsWindow, TopWidget};

use super::{
    lock, open_settings, quit, request_refresh, save_settings, set_top_visible, start_update_check,
    start_update_install, toggle_top_lock, toggle_top_pin, Backend, UiBridge,
};

pub(super) fn wire_callbacks(bridge: &UiBridge, backend: &Backend) {
    wire_top_callbacks(bridge, backend);
    wire_detail_callbacks(bridge, backend);
    wire_settings_callbacks(bridge, backend);
    wire_tray_callbacks(bridge, backend);
}

fn wire_top_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(top) = bridge.top.upgrade() else {
        return;
    };
    bind_refresh(&top, bridge, backend);
    let ui = bridge.clone();
    top.on_open_detail(move || ui.toggle_detail());
    let ui = bridge.clone();
    let state = backend.clone();
    top.on_hide_window(move || set_top_visible(&ui, &state, false));
    let weak = top.as_weak();
    let config = backend.config.clone();
    top.on_start_drag(move || {
        if !lock(&config).top_lock_position {
            if let Some(window) = weak.upgrade() {
                windows::begin_horizontal_drag(window.window());
            }
        }
    });
    let ui = bridge.clone();
    let state = backend.clone();
    top.on_toggle_pin(move || toggle_top_pin(&ui, &state));
    let ui = bridge.clone();
    let state = backend.clone();
    top.on_toggle_lock(move || toggle_top_lock(&ui, &state));
}

fn wire_detail_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(detail) = bridge.detail.upgrade() else {
        return;
    };
    bind_refresh(&detail, bridge, backend);
    let weak = detail.as_weak();
    detail.on_close_window(move || {
        if let Some(window) = weak.upgrade() {
            let _ = window.hide();
        }
    });
    let weak = detail.as_weak();
    detail.on_start_drag(move || {
        if let Some(window) = weak.upgrade() {
            windows::begin_window_drag(window.window());
        }
    });
}

fn wire_settings_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(settings) = bridge.settings.upgrade() else {
        return;
    };
    let weak = settings.as_weak();
    settings.on_close_window(move || {
        if let Some(window) = weak.upgrade() {
            let _ = window.hide();
        }
    });
    let weak = settings.as_weak();
    settings.on_start_drag(move || {
        if let Some(window) = weak.upgrade() {
            windows::begin_window_drag(window.window());
        }
    });
    let ui = bridge.clone();
    settings.on_open_detail(move || ui.toggle_detail());
    bind_refresh(&settings, bridge, backend);

    let weak = settings.as_weak();
    let ui = bridge.clone();
    let state = backend.clone();
    settings.on_save_settings(move || {
        if let Some(window) = weak.upgrade() {
            save_settings(&window, &ui, &state);
        }
    });
    let ui = bridge.clone();
    let state = backend.clone();
    settings.on_check_update(move || start_update_check(ui.clone(), state.clone()));
    let ui = bridge.clone();
    let state = backend.clone();
    settings.on_install_update(move || start_update_install(ui.clone(), state.clone()));
    let state = backend.clone();
    settings.on_quit_app(move || quit(&state));
}

fn wire_tray_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(tray) = bridge.tray.upgrade() else {
        return;
    };
    let last_click = Arc::new(Mutex::new(None::<Instant>));
    let ui = bridge.clone();
    tray.on_show_all(move || {
        let now = Instant::now();
        let mut last = lock(&last_click);
        if last.is_some_and(|value| now.duration_since(value) < Duration::from_millis(450)) {
            ui.bring_visible_to_front();
            *last = None;
        } else {
            *last = Some(now);
        }
    });
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_toggle_top(move || {
        let visible = ui
            .top
            .upgrade()
            .is_some_and(|window| window.window().is_visible());
        set_top_visible(&ui, &state, !visible);
    });
    let ui = bridge.clone();
    tray.on_open_detail(move || ui.toggle_detail());
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_open_settings(move || open_settings(&ui, &state));
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_refresh(move || request_refresh(ui.clone(), state.clone()));
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_toggle_top_pin(move || toggle_top_pin(&ui, &state));
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_toggle_top_lock(move || toggle_top_lock(&ui, &state));
    let ui = bridge.clone();
    let state = backend.clone();
    tray.on_update_action(move || {
        if lock(&state.update).is_some() {
            start_update_install(ui.clone(), state.clone());
        } else {
            start_update_check(ui.clone(), state.clone());
        }
    });
    let state = backend.clone();
    tray.on_quit_app(move || quit(&state));
}

trait RefreshCallback {
    fn on_refresh(&self, callback: impl Fn() + 'static);
}

macro_rules! impl_refresh_callback {
    ($($type:ty),+ $(,)?) => {$(
        impl RefreshCallback for $type {
            fn on_refresh(&self, callback: impl Fn() + 'static) {
                self.on_refresh(callback);
            }
        }
    )+};
}

impl_refresh_callback!(TopWidget, DetailWindow, SettingsWindow);

fn bind_refresh(component: &impl RefreshCallback, bridge: &UiBridge, backend: &Backend) {
    let ui = bridge.clone();
    let state = backend.clone();
    component.on_refresh(move || request_refresh(ui.clone(), state.clone()));
}
