use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

use crate::AppState;

pub const COLLAPSED_WIDTH: u32 = 360;
pub const COLLAPSED_HEIGHT: u32 = 142;
pub const EXPANDED_WIDTH: u32 = 460;
pub const EXPANDED_HEIGHT: u32 = 640;

const DEFAULT_TOP_MARGIN: i32 = 28;

pub fn main_window_size(expanded: bool) -> LogicalSize<u32> {
    if expanded {
        LogicalSize::new(EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        LogicalSize::new(COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    }
}

pub fn setup_main_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let state = app.state::<AppState>();
    let config = state.config.lock().expect("config mutex").clone();

    window.set_size(main_window_size(false))?;
    let _ = window.set_shadow(false);
    window.set_always_on_top(config.general.always_on_top)?;
    place_main_window(&window, config.window.x, config.window.y);
    if !config.general.show_on_startup {
        let _ = window.hide();
    }

    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        WindowEvent::Moved(position) => {
            let state = app_handle.state::<AppState>();
            state.save_window_position(position.x, position.y);
        }
        _ => {}
    });

    Ok(())
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
