use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow, WindowEvent,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use crate::AppState;

pub const COLLAPSED_WIDTH: u32 = 430;
pub const COLLAPSED_HEIGHT: u32 = 104;
pub const EXPANDED_WIDTH: u32 = 460;
pub const EXPANDED_HEIGHT: u32 = 640;
pub const TOP_WIDTH: u32 = 168;
pub const TOP_HEIGHT: u32 = 26;
pub const TOP_MENU_HEIGHT: u32 = 108;
pub const MENUBAR_WIDTH: u32 = 360;
pub const MENUBAR_HEIGHT: u32 = 480;

const DEFAULT_TOP_MARGIN: i32 = 28;
const TOP_STATUS_MARGIN: i32 = 0;
const HIDDEN_WINDOW_COORDINATE: i32 = -32_000;

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
    hide_from_windows_taskbar(&window);
    window.set_always_on_top(config.general.main_always_on_top)?;
    place_main_window(&window, config.window.x, config.window.y);
    if config.general.show_on_startup {
        let _ = show_window_without_taskbar(&window);
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
    hide_from_windows_taskbar(&window);
    window.set_always_on_top(config.general.top_always_on_top)?;
    place_top_window(&window, config.window.top_x);
    if config.general.top_status_enabled {
        let _ = show_window_without_taskbar(&window);
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
        hide_from_windows_taskbar(&window);

        let panel = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = panel.hide();
            }
        });
    }
}

pub fn setup_menubar_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("menubar") else {
        return Ok(());
    };

    window.set_size(LogicalSize::new(MENUBAR_WIDTH, MENUBAR_HEIGHT))?;
    window.set_always_on_top(true)?;
    let _ = window.set_shadow(true);

    #[cfg(target_os = "macos")]
    let app_handle = app.clone();
    let panel = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = panel.hide();
        }
        WindowEvent::Focused(false) => {
            #[cfg(target_os = "macos")]
            app_handle.state::<AppState>().mark_menubar_blurred();
            let _ = panel.hide();
        }
        _ => {}
    });

    Ok(())
}

pub fn show_menubar_window(app: &AppHandle, anchor: Option<(f64, f64)>) {
    let Some(window) = app.get_webview_window("menubar") else {
        return;
    };

    place_menubar_window(&window, anchor);
    let _ = window.show();
    let _ = window.set_focus();
}

pub(crate) fn show_window_without_taskbar(window: &WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    hide_from_windows_taskbar(window);
    Ok(())
}

fn place_menubar_window(window: &WebviewWindow, anchor: Option<(f64, f64)>) {
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = anchor
        .and_then(|(x, y)| {
            monitors.iter().find(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                x >= origin.x as f64
                    && x < (origin.x + size.width as i32) as f64
                    && y >= origin.y as f64
                    && y < (origin.y + size.height as i32) as f64
            })
        })
        .or_else(|| monitors.first());
    let Some(monitor) = monitor else {
        return;
    };

    let origin = monitor.position();
    let size = monitor.size();
    let scale = monitor.scale_factor();
    let panel_width = (MENUBAR_WIDTH as f64 * scale).round() as i32;
    let panel_height = (MENUBAR_HEIGHT as f64 * scale).round() as i32;
    let inset = (8.0 * scale).round() as i32;
    let menu_bar_bottom = origin.y + (24.0 * scale).round() as i32;
    let min_x = origin.x + inset;
    let max_x = (origin.x + size.width as i32 - panel_width - inset).max(min_x);
    let max_y = (origin.y + size.height as i32 - panel_height - inset).max(menu_bar_bottom);

    let anchor_x = anchor
        .map(|(x, _)| x.round() as i32)
        .unwrap_or(origin.x + size.width as i32 - panel_width / 2 - inset);
    let anchor_y = anchor
        .map(|(_, y)| y.round() as i32 + (12.0 * scale).round() as i32)
        .unwrap_or(menu_bar_bottom);
    let x = (anchor_x - panel_width / 2).clamp(min_x, max_x);
    let y = anchor_y.max(menu_bar_bottom).min(max_y);

    let _ = window.set_position(PhysicalPosition::new(x, y));
}

#[cfg(windows)]
fn hide_from_windows_taskbar(window: &WebviewWindow) {
    // Tao 的 skipTaskbar 只会通知任务栏，仍需调整窗口样式以移除任务栏按钮。
    let _ = window.set_skip_taskbar(true);
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next_style =
            (current_style & !(WS_EX_APPWINDOW.0 as isize)) | WS_EX_TOOLWINDOW.0 as isize;
        if current_style == next_style {
            return;
        }

        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(windows))]
fn hide_from_windows_taskbar(_window: &WebviewWindow) {}

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
    let saved_x = saved_x.filter(|position| !is_hidden_window_coordinate(*position));
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = saved_x
        .and_then(|position| {
            monitors
                .iter()
                .find(|monitor| {
                    let origin = monitor.position();
                    let width = monitor.size().width as i32;
                    position >= origin.x && position < origin.x.saturating_add(width)
                })
                .cloned()
        })
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| monitors.first().cloned());
    let Some(monitor) = monitor else {
        return;
    };

    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let physical_width = (TOP_WIDTH as f64 * monitor.scale_factor()).round() as u32;
    let min_x = monitor_origin.x;
    let max_x = monitor_origin.x + monitor_size.width.saturating_sub(physical_width) as i32;
    let x = saved_x
        .filter(|position| (*position >= min_x) && (*position <= max_x))
        .unwrap_or_else(|| {
            min_x + ((monitor_size.width.saturating_sub(physical_width)) / 2) as i32
        });
    let y = monitor_origin.y + TOP_STATUS_MARGIN;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub(crate) fn is_hidden_window_position(x: i32, y: i32) -> bool {
    is_hidden_window_coordinate(x) || is_hidden_window_coordinate(y)
}

pub(crate) fn is_hidden_window_coordinate(position: i32) -> bool {
    // Windows 隐藏窗口时可能发送 -32000 的临时坐标，不能写入用户位置配置。
    position <= HIDDEN_WINDOW_COORDINATE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_windows_hidden_window_coordinate() {
        assert!(is_hidden_window_coordinate(-32_000));
        assert!(is_hidden_window_position(120, -32_000));
        assert!(!is_hidden_window_position(-10_000, 0));
    }
}
