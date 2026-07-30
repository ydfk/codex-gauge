use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use slint::{ComponentHandle, Timer, TimerMode};

mod callbacks;
mod ui_bridge;
mod update_flow;

use callbacks::wire_callbacks;
use ui_bridge::{set_logical_size, style_later, UiBridge};
use update_flow::{start_update_check, start_update_install};

use crate::{
    codex,
    config::{AppConfig, ProviderPreference},
    model::{CodexUsageSnapshot, SnapshotSource, SnapshotStatus},
    storage::{AppStorage, StateDocument},
    updater::UpdateInfo,
    windows, AppTray, DetailWindow, MainWidget, SettingsWindow, TopWidget,
};

#[derive(Clone)]
struct Backend {
    config: Arc<Mutex<AppConfig>>,
    state: Arc<Mutex<StateDocument>>,
    storage: AppStorage,
    refreshing: Arc<AtomicBool>,
    exiting: Arc<AtomicBool>,
    update: Arc<Mutex<Option<UpdateInfo>>>,
}

pub fn run() -> Result<(), slint::PlatformError> {
    let storage = AppStorage::new();
    let mut config = storage.load_config();
    config.start_on_boot = windows::autostart_enabled();
    let state = storage.load_state();

    let main = MainWidget::new()?;
    let top = TopWidget::new()?;
    let detail = DetailWindow::new()?;
    let settings = SettingsWindow::new()?;
    let tray = AppTray::new()?;
    let bridge = UiBridge {
        main: main.as_weak(),
        top: top.as_weak(),
        detail: detail.as_weak(),
        settings: settings.as_weak(),
        tray: tray.as_weak(),
    };
    let backend = Backend {
        config: Arc::new(Mutex::new(config.clone())),
        state: Arc::new(Mutex::new(state)),
        storage,
        refreshing: Arc::new(AtomicBool::new(false)),
        exiting: Arc::new(AtomicBool::new(false)),
        update: Arc::new(Mutex::new(None)),
    };

    let last_snapshot = lock(&backend.state)
        .last_snapshot
        .clone()
        .unwrap_or_else(|| {
            CodexUsageSnapshot::empty(SnapshotSource::AppServer, SnapshotStatus::RequestFailed)
        });
    bridge.apply_snapshot(&last_snapshot);
    configure_initial_windows(&bridge, &config);
    wire_callbacks(&bridge, &backend);
    bridge.sync_config(&config);

    tray.show()?;
    style_later(detail.as_weak(), false, 460.0, 520.0);
    style_later(settings.as_weak(), false, 700.0, 520.0);
    if config.show_main_on_startup {
        bridge.show_main();
    }
    if config.show_top_on_startup {
        bridge.show_top();
    }

    let position_timer = start_position_persistence(bridge.clone(), backend.clone());
    let oled_timer = start_oled_shift(bridge.clone(), backend.clone());
    let top_hover_timer = start_top_hover(bridge.clone());
    start_refresh_loop(bridge.clone(), backend.clone());
    if config.update.check_on_startup {
        start_update_check(bridge.clone(), backend.clone());
    }

    let result = slint::run_event_loop_until_quit();
    backend.exiting.store(true, Ordering::Release);
    drop(position_timer);
    drop(oled_timer);
    drop(top_hover_timer);
    result
}

fn configure_initial_windows(bridge: &UiBridge, config: &AppConfig) {
    if let Some(main) = bridge.main.upgrade() {
        set_logical_size(main.window(), 430.0, 104.0);
        let position = match (config.windows.main_x, config.windows.main_y) {
            (Some(x), Some(y)) if windows::valid_saved_position(x, y) => (x, y),
            _ => {
                let (width, height) = windows::scaled_size(430, 104);
                windows::default_main_position(width, height)
            }
        };
        windows::set_position(main.window(), position.0, position.1);
        main.set_pinned(config.main_always_on_top);
        main.set_locked(config.main_lock_position);
        main.set_panel_opacity(config.opacity);
    }
    if let Some(top) = bridge.top.upgrade() {
        let logical_width = if top.get_data().five_visible {
            220.0
        } else {
            170.0
        };
        set_logical_size(top.window(), logical_width, 28.0);
        let (physical_width, _) = windows::scaled_size(logical_width as i32, 28);
        let default = windows::default_top_position(physical_width);
        let x = config
            .windows
            .top_x
            .filter(|x| windows::valid_saved_position(*x, 0))
            .unwrap_or(default.0);
        windows::set_position(top.window(), x, default.1);
        top.set_pinned(config.top_always_on_top);
        top.set_locked(config.top_lock_position);
        top.set_panel_opacity(config.opacity);
    }
    center_panel_windows(bridge);
}

fn center_panel_windows(bridge: &UiBridge) {
    if let Some(detail) = bridge.detail.upgrade() {
        set_logical_size(detail.window(), 460.0, 520.0);
        let (width, height) = windows::scaled_size(460, 520);
        let (x, y) = windows::default_main_position(width, height);
        windows::set_position(detail.window(), x, y);
    }
    if let Some(settings) = bridge.settings.upgrade() {
        set_logical_size(settings.window(), 700.0, 520.0);
        let (width, height) = windows::scaled_size(700, 520);
        let (x, y) = windows::default_main_position(width, height);
        windows::set_position(settings.window(), x, y);
    }
}

fn request_refresh(bridge: UiBridge, backend: Backend) {
    if backend.refreshing.swap(true, Ordering::AcqRel) {
        return;
    }
    thread::spawn(move || {
        let config = lock(&backend.config).clone();
        let current = codex::refresh_snapshot(&config);
        let display = merge_with_last_success(current, &backend);
        backend.refreshing.store(false, Ordering::Release);
        let _ = slint::invoke_from_event_loop(move || bridge.apply_snapshot(&display));
    });
}

fn merge_with_last_success(
    mut snapshot: CodexUsageSnapshot,
    backend: &Backend,
) -> CodexUsageSnapshot {
    let mut state = lock(&backend.state);
    let has_usage = snapshot.primary_window.is_some() || snapshot.secondary_window.is_some();
    if snapshot.status != SnapshotStatus::Ok && !has_usage {
        if let Some(last) = state.last_snapshot.as_ref() {
            snapshot.primary_window = last.primary_window.clone();
            snapshot.primary_window_unlimited = last.primary_window_unlimited;
            snapshot.secondary_window = last.secondary_window.clone();
            snapshot.credits = snapshot.credits.or_else(|| last.credits.clone());
            snapshot.plan_type = snapshot.plan_type.or_else(|| last.plan_type.clone());
        }
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    } else {
        state.consecutive_failures = 0;
        state.last_success_at = Some(snapshot.updated_at);
    }
    state.last_snapshot = Some(snapshot.clone());
    backend.storage.save_state(&state);
    snapshot
}

fn start_refresh_loop(bridge: UiBridge, backend: Backend) {
    thread::spawn(move || {
        request_refresh(bridge.clone(), backend.clone());
        while !backend.exiting.load(Ordering::Acquire) {
            let failures = lock(&backend.state).consecutive_failures;
            let configured = lock(&backend.config).refresh_interval_seconds;
            let delay = match failures {
                0 => configured,
                1 => 30,
                2 => 60,
                3 => 120,
                _ => 300,
            };
            thread::sleep(Duration::from_secs(delay));
            if !backend.exiting.load(Ordering::Acquire) {
                request_refresh(bridge.clone(), backend.clone());
            }
        }
    });
}

fn start_position_persistence(bridge: UiBridge, backend: Backend) -> Timer {
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(2), move || {
        let mut config = lock(&backend.config);
        let mut changed = false;
        if let Some(main) = bridge.main.upgrade() {
            if main.window().is_visible() {
                if let Some((x, y)) = windows::position(main.window()) {
                    if windows::valid_saved_position(x, y)
                        && (config.windows.main_x != Some(x) || config.windows.main_y != Some(y))
                    {
                        config.windows.main_x = Some(x);
                        config.windows.main_y = Some(y);
                        changed = true;
                    }
                }
            }
        }
        if let Some(top) = bridge.top.upgrade() {
            if top.window().is_visible() {
                if let Some((x, _)) = windows::position(top.window()) {
                    if windows::valid_saved_position(x, 0) && config.windows.top_x != Some(x) {
                        config.windows.top_x = Some(x);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            backend.storage.save_config(&config);
        }
    });
    timer
}

fn start_oled_shift(bridge: UiBridge, backend: Backend) -> Timer {
    let timer = Timer::default();
    let step = Arc::new(Mutex::new(0usize));
    timer.start(TimerMode::Repeated, Duration::from_secs(300), move || {
        if !lock(&backend.config).oled_shift_enabled {
            return;
        }
        let offsets = [(-1, 0), (1, 0)];
        let mut index = lock(&step);
        *index = (*index + 1) % offsets.len();
        let (dx, dy) = offsets[*index];
        if let Some(main) = bridge.main.upgrade() {
            if let Some((x, y)) = windows::position(main.window()) {
                windows::set_position(main.window(), x + dx, y + dy);
            }
        }
        if let Some(top) = bridge.top.upgrade() {
            if let Some((x, _)) = windows::position(top.window()) {
                windows::set_position(top.window(), x + dx, 0);
            }
        }
    });
    timer
}

fn start_top_hover(bridge: UiBridge) -> Timer {
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if let Some(top) = bridge.top.upgrade() {
            let expanded = top.window().is_visible() && windows::cursor_inside(top.window());
            if top.get_expanded() != expanded {
                top.set_expanded(expanded);
            }
        }
    });
    timer
}

fn save_settings(window: &SettingsWindow, bridge: &UiBridge, backend: &Backend) {
    let mut config = lock(&backend.config);
    config.start_on_boot = window.get_start_on_boot();
    config.show_main_on_startup = window.get_show_main();
    config.show_top_on_startup = window.get_show_top();
    config.main_always_on_top = window.get_main_pinned();
    config.top_always_on_top = window.get_top_pinned();
    config.main_lock_position = window.get_main_locked();
    config.top_lock_position = window.get_top_locked();
    config.oled_shift_enabled = window.get_oled_shift();
    config.opacity = (window.get_panel_opacity() / 100.0).clamp(0.68, 1.0);
    config.refresh_interval_seconds = window
        .get_refresh_seconds()
        .parse::<u64>()
        .unwrap_or(config.refresh_interval_seconds)
        .clamp(30, 3600);
    config.codex_command = window.get_codex_command().trim().to_string();
    config.preferred_provider = if window.get_provider().eq_ignore_ascii_case("api") {
        ProviderPreference::Api
    } else {
        ProviderPreference::AppServer
    };
    config.update.check_on_startup = window.get_update_on_start();
    config.update.endpoint = window.get_update_endpoint().trim().to_string();
    config.normalize();
    let saved = config.clone();
    backend.storage.save_config(&saved);
    drop(config);

    let _ = windows::set_autostart(saved.start_on_boot);
    bridge.sync_config(&saved);
    set_main_visible(bridge, backend, saved.show_main_on_startup);
    set_top_visible(bridge, backend, saved.show_top_on_startup);
    let _ = window.hide();
}

fn open_settings(bridge: &UiBridge, backend: &Backend) {
    let config = lock(&backend.config).clone();
    let Some(settings) = bridge.settings.upgrade() else {
        return;
    };
    settings.set_start_on_boot(windows::autostart_enabled());
    settings.set_show_main(config.show_main_on_startup);
    settings.set_show_top(config.show_top_on_startup);
    settings.set_main_pinned(config.main_always_on_top);
    settings.set_top_pinned(config.top_always_on_top);
    settings.set_main_locked(config.main_lock_position);
    settings.set_top_locked(config.top_lock_position);
    settings.set_oled_shift(config.oled_shift_enabled);
    settings.set_panel_opacity(config.opacity * 100.0);
    settings.set_refresh_seconds(config.refresh_interval_seconds.to_string().into());
    settings.set_codex_command(config.codex_command.into());
    settings.set_provider(
        match config.preferred_provider {
            ProviderPreference::AppServer => "app-server",
            ProviderPreference::Api => "api",
        }
        .into(),
    );
    settings.set_update_on_start(config.update.check_on_startup);
    settings.set_update_endpoint(config.update.endpoint.into());
    settings.set_version_text(format!("v{}", env!("CARGO_PKG_VERSION")).into());
    set_logical_size(settings.window(), 700.0, 520.0);
    let _ = settings.show();
    style_later(settings.as_weak(), false, 700.0, 520.0);
    windows::bring_to_front(settings.window());
}

fn set_main_visible(bridge: &UiBridge, backend: &Backend, visible: bool) {
    lock(&backend.config).show_main_on_startup = visible;
    backend.storage.save_config(&lock(&backend.config));
    if visible {
        bridge.show_main();
    } else if let Some(main) = bridge.main.upgrade() {
        let _ = main.hide();
    }
    bridge.sync_visibility();
}

fn set_top_visible(bridge: &UiBridge, backend: &Backend, visible: bool) {
    lock(&backend.config).show_top_on_startup = visible;
    backend.storage.save_config(&lock(&backend.config));
    if visible {
        bridge.show_top();
    } else if let Some(top) = bridge.top.upgrade() {
        let _ = top.hide();
    }
    bridge.sync_visibility();
}

fn toggle_main_pin(bridge: &UiBridge, backend: &Backend) {
    let mut config = lock(&backend.config);
    config.main_always_on_top = !config.main_always_on_top;
    backend.storage.save_config(&config);
    bridge.sync_config(&config);
}

fn toggle_top_pin(bridge: &UiBridge, backend: &Backend) {
    let mut config = lock(&backend.config);
    config.top_always_on_top = !config.top_always_on_top;
    backend.storage.save_config(&config);
    bridge.sync_config(&config);
}

fn toggle_main_lock(bridge: &UiBridge, backend: &Backend) {
    let mut config = lock(&backend.config);
    config.main_lock_position = !config.main_lock_position;
    backend.storage.save_config(&config);
    bridge.sync_config(&config);
}

fn toggle_top_lock(bridge: &UiBridge, backend: &Backend) {
    let mut config = lock(&backend.config);
    config.top_lock_position = !config.top_lock_position;
    backend.storage.save_config(&config);
    bridge.sync_config(&config);
}

fn quit(backend: &Backend) {
    backend.exiting.store(true, Ordering::Release);
    let _ = slint::quit_event_loop();
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
