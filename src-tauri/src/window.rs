use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow, WindowEvent,
};

use crate::AppState;

pub const COLLAPSED_WIDTH: u32 = 430;
pub const COLLAPSED_HEIGHT: u32 = 104;
pub const EXPANDED_WIDTH: u32 = 460;
pub const EXPANDED_HEIGHT: u32 = 640;
pub const TOP_WIDTH: u32 = 168;
pub const TOP_HEIGHT: u32 = 26;
pub const TOP_MENU_HEIGHT: u32 = 108;

const DEFAULT_TOP_MARGIN: i32 = 28;
const TOP_STATUS_MARGIN: i32 = 0;

pub fn main_window_size(expanded: bool) -> LogicalSize<u32> {
    if expanded {
        LogicalSize::new(EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        LogicalSize::new(COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    }
}

pub fn top_window_size(menu_open: bool) -> LogicalSize<u32> {
    LogicalSize::new(
        TOP_WIDTH,
        if menu_open {
            TOP_MENU_HEIGHT
        } else {
            TOP_HEIGHT
        },
    )
}

pub fn setup_main_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let state = app.state::<AppState>();
    let config = state.config.lock().expect("config mutex").clone();

    window.set_size(main_window_size(false))?;
    let _ = window.set_shadow(false);
    window.set_always_on_top(config.general.main_always_on_top)?;
    place_main_window(&window, config.window.x, config.window.y);
    if config.general.show_on_startup {
        let _ = window.show();
    }

    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
            }
            let state = app_handle.state::<AppState>();
            if state.set_window_visibility_preference("main", false) {
                let _ = app_handle.emit("codex-gauge-config-updated", ());
            }
        }
        WindowEvent::Moved(position) => {
            let state = app_handle.state::<AppState>();
            if !state.is_oled_move("main", position.x, position.y) {
                state.save_window_position(position.x, position.y);
            }
        }
        _ => {}
    });

    Ok(())
}

pub fn setup_top_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("top") else {
        return Ok(());
    };

    let state = app.state::<AppState>();
    let config = state.config.lock().expect("config mutex").clone();

    window.set_size(top_window_size(false))?;
    let _ = window.set_shadow(false);
    window.set_always_on_top(config.general.top_always_on_top)?;
    place_top_window(&window, config.window.top_x);
    if config.general.top_status_enabled {
        let _ = window.show();
    }

    let app_handle = app.clone();
    let top_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = top_window.hide();
            let state = app_handle.state::<AppState>();
            if state.set_window_visibility_preference("top", false) {
                let _ = app_handle.emit("codex-gauge-config-updated", ());
            }
        }
        WindowEvent::Moved(position) => {
            let state = app_handle.state::<AppState>();
            if !state.is_oled_move("top", position.x, position.y) {
                state.save_top_window_position(position.x);
            }
        }
        _ => {}
    });

    Ok(())
}

pub fn setup_panel_windows(app: &AppHandle) {
    for label in ["detail", "settings"] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        let _ = window.set_shadow(false);

        let panel = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = panel.hide();
            }
        });
    }
}

fn place_main_window(window: &WebviewWindow, x: Option<i32>, y: Option<i32>) {
    if let (Some(x), Some(y)) = (x, y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
        return;
    }

    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };

    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let physical_width = (COLLAPSED_WIDTH as f64 * monitor.scale_factor()).round() as u32;
    let x = monitor_origin.x + ((monitor_size.width.saturating_sub(physical_width)) / 2) as i32;
    let y = monitor_origin.y + DEFAULT_TOP_MARGIN;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn place_top_window(window: &WebviewWindow, saved_x: Option<i32>) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };

    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let physical_width = (TOP_WIDTH as f64 * monitor.scale_factor()).round() as u32;
    let x = saved_x.unwrap_or_else(|| {
        monitor_origin.x + ((monitor_size.width.saturating_sub(physical_width)) / 2) as i32
    });
    let y = monitor_origin.y + TOP_STATUS_MARGIN;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
