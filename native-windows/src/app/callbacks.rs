use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use slint::ComponentHandle;

use crate::{windows, PanelWindow, TopWidget};

use super::{
    lock, open_codex_login, open_settings, open_usage, quit, request_refresh, save_settings,
    set_top_visible, start_update_check, start_update_install, toggle_top_lock, toggle_top_pin,
    Backend, UiBridge,
};

pub(super) fn wire_callbacks(bridge: &UiBridge, backend: &Backend) {
    wire_top_callbacks(bridge, backend);
    wire_panel_callbacks(bridge, backend);
    wire_tray_callbacks(bridge, backend);
}

fn wire_top_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(top) = bridge.top.upgrade() else {
        return;
    };
    bind_refresh(&top, bridge, backend);

    let last_click = Arc::new(Mutex::new(None::<Instant>));
    let ui = bridge.clone();
    top.on_toggle_panel(move || {
        let now = Instant::now();
        let mut last = lock(&last_click);
        if last.is_some_and(|value| now.duration_since(value) < Duration::from_millis(250)) {
            return;
        }
        *last = Some(now);
        ui.toggle_panel();
    });

    let ui = bridge.clone();
    let state = backend.clone();
    top.on_hide_window(move || set_top_visible(&ui, &state, false));
    let weak = top.as_weak();
    let panel_weak = bridge.panel.clone();
    let config = backend.config.clone();
    top.on_start_drag(move || {
        if !lock(&config).top_lock_position {
            if let Some(window) = weak.upgrade() {
                let panel = panel_weak
                    .upgrade()
                    .filter(|panel| panel.window().is_visible());
                windows::begin_horizontal_drag(
                    window.window(),
                    panel.as_ref().map(|panel| panel.window()),
                );
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

fn wire_panel_callbacks(bridge: &UiBridge, backend: &Backend) {
    let Some(panel) = bridge.panel.upgrade() else {
        return;
    };
    bind_refresh(&panel, bridge, backend);

    let weak = panel.as_weak();
    let ui = bridge.clone();
    let state = backend.clone();
    panel.on_close_window(move || {
        if let Some(window) = weak.upgrade() {
            save_settings(&window, &ui, &state);
            let _ = window.hide();
        }
    });

    let ui = bridge.clone();
    let state = backend.clone();
    panel.on_open_login(move || open_codex_login(ui.clone(), state.clone()));

    let weak = panel.as_weak();
    let ui = bridge.clone();
    let state = backend.clone();
    panel.on_settings_changed(move || {
        if let Some(window) = weak.upgrade() {
            save_settings(&window, &ui, &state);
        }
    });

    let ui = bridge.clone();
    let state = backend.clone();
    panel.on_check_update(move || start_update_check(ui.clone(), state.clone()));
    let ui = bridge.clone();
    let state = backend.clone();
    panel.on_install_update(move || start_update_install(ui.clone(), state.clone()));
    let state = backend.clone();
    panel.on_quit_app(move || quit(&state));
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
    tray.on_open_detail(move || open_usage(&ui));
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

impl_refresh_callback!(TopWidget, PanelWindow);

fn bind_refresh(component: &impl RefreshCallback, bridge: &UiBridge, backend: &Backend) {
    let ui = bridge.clone();
    let state = backend.clone();
    component.on_refresh(move || request_refresh(ui.clone(), state.clone()));
}
