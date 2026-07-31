use std::{ffi::c_void, thread, time::Duration};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{PhysicalPosition, Window, WindowPosition};
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{
            DwmSetWindowAttribute, DWMSBT_NONE, DWMWA_BORDER_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
            DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
            DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE,
        },
        Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    },
    UI::{
        HiDpi::{
            GetDpiForSystem, SetProcessDpiAwarenessContext,
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        },
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        WindowsAndMessaging::{
            GetCursorPos, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, SetForegroundWindow,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST,
            SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SW_SHOW, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        },
    },
};

pub fn initialize_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn apply_native_style(window: &Window, topmost: bool) -> Result<(), String> {
    let hwnd = hwnd(window)?;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let style = (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize);
        SetWindowPos(
            hwnd,
            Some(if topmost {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(|_| "window_style")?;

        let dark_mode = 1i32;
        let border_color = 0xffff_fffeu32;
        let backdrop = DWMSBT_NONE;
        let corner = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_mode as *const i32).cast(),
            size_of_val(&dark_mode) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&border_color as *const u32).cast(),
            size_of_val(&border_color) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&backdrop as *const DWM_SYSTEMBACKDROP_TYPE).cast(),
            size_of_val(&backdrop) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            size_of_val(&corner) as u32,
        );
    }
    Ok(())
}

pub fn scaled_size(width: i32, height: i32) -> (i32, i32) {
    let dpi = unsafe { GetDpiForSystem() }.max(96) as i32;
    (width * dpi / 96, height * dpi / 96)
}

pub fn begin_horizontal_drag(window: &Window, panel: Option<&Window>) {
    let Ok(hwnd) = hwnd(window) else {
        return;
    };
    let mut rect = RECT::default();
    let mut cursor = POINT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err()
        || unsafe { GetCursorPos(&mut cursor) }.is_err()
    {
        return;
    }

    let hwnd_value = hwnd.0 as isize;
    let start_cursor_x = cursor.x;
    let start_window_x = rect.left;
    let fixed_y = rect.top;
    let window_width = rect.right - rect.left;
    let panel_state = panel.and_then(window_state);
    thread::spawn(move || {
        let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let right = left + unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        while unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0 {
            let mut point = POINT::default();
            if unsafe { GetCursorPos(&mut point) }.is_ok() {
                let x = (start_window_x + point.x - start_cursor_x)
                    .clamp(left, (right - window_width).max(left));
                let hwnd = HWND(hwnd_value as *mut c_void);
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        x,
                        fixed_y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
                if let Some(panel) = panel_state {
                    let panel_x =
                        (panel.x + x - start_window_x).clamp(left, (right - panel.width).max(left));
                    let panel_hwnd = HWND(panel.hwnd as *mut c_void);
                    let _ = unsafe {
                        SetWindowPos(
                            panel_hwnd,
                            None,
                            panel_x,
                            panel.y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        )
                    };
                }
            }
            thread::sleep(Duration::from_millis(12));
        }
    });
}

#[derive(Clone, Copy)]
struct WindowState {
    hwnd: isize,
    x: i32,
    y: i32,
    width: i32,
}

fn window_state(window: &Window) -> Option<WindowState> {
    let hwnd = hwnd(window).ok()?;
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(WindowState {
        hwnd: hwnd.0 as isize,
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(1),
    })
}

pub fn bring_to_front(window: &Window) {
    if let Ok(hwnd) = hwnd(window) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

pub fn position(window: &Window) -> Option<(i32, i32)> {
    let hwnd = hwnd(window).ok()?;
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some((rect.left, rect.top))
}

pub fn set_position(window: &Window, x: i32, y: i32) {
    window.set_position(WindowPosition::Physical(PhysicalPosition::new(x, y)));
}

pub fn place_below(anchor: &Window, panel: &Window) {
    let (Ok(anchor_hwnd), Ok(panel_hwnd)) = (hwnd(anchor), hwnd(panel)) else {
        return;
    };
    let mut anchor_rect = RECT::default();
    let mut panel_rect = RECT::default();
    if unsafe { GetWindowRect(anchor_hwnd, &mut anchor_rect) }.is_err()
        || unsafe { GetWindowRect(panel_hwnd, &mut panel_rect) }.is_err()
    {
        return;
    }

    let monitor = unsafe { MonitorFromWindow(anchor_hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        let position = clamp_panel_position(anchor_rect, info.rcWork, panel_rect);
        set_position(panel, position.0, position.1);
    }
}

pub fn default_main_position(width: i32, height: i32) -> (i32, i32) {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    ((screen_width - width) / 2, (screen_height - height) / 2)
}

pub fn default_top_position(width: i32) -> (i32, i32) {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    ((screen_width - width) / 2, 0)
}

pub fn valid_saved_position(x: i32, y: i32) -> bool {
    if x == -32_000 || y == -32_000 {
        return false;
    }
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let right = left + unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let bottom = top + unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    x >= left - 64 && x < right && y >= top - 64 && y < bottom
}

fn clamp_panel_position(anchor: RECT, work: RECT, panel: RECT) -> (i32, i32) {
    let panel_width = (panel.right - panel.left).max(1);
    let panel_height = (panel.bottom - panel.top).max(1);
    let centered_x = anchor.left + (anchor.right - anchor.left - panel_width) / 2;
    let x = centered_x.clamp(work.left, (work.right - panel_width).max(work.left));
    let below = anchor.bottom - 1;
    let y = if below + panel_height <= work.bottom {
        below
    } else {
        (work.bottom - panel_height).max(work.top)
    };
    (x, y)
}

fn hwnd(window: &Window) -> Result<HWND, String> {
    let handle = window.window_handle();
    let raw = handle
        .window_handle()
        .map_err(|_| "window_handle")?
        .as_raw();
    match raw {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut c_void)),
        _ => Err("window_handle".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_windows_hidden_coordinate() {
        assert!(!valid_saved_position(-32_000, -32_000));
    }

    #[test]
    fn panel_is_centered_below_anchor_and_clamped_to_work_area() {
        let anchor = RECT {
            left: 900,
            top: 0,
            right: 1_000,
            bottom: 22,
        };
        let work = RECT {
            left: 0,
            top: 0,
            right: 1_920,
            bottom: 1_040,
        };
        let panel = RECT {
            left: 0,
            top: 0,
            right: 376,
            bottom: 510,
        };

        assert_eq!(clamp_panel_position(anchor, work, panel), (762, 21));

        let edge_anchor = RECT {
            left: 0,
            right: 40,
            ..anchor
        };
        assert_eq!(clamp_panel_position(edge_anchor, work, panel), (0, 21));
    }
}
