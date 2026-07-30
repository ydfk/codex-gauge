use std::{ffi::c_void, thread, time::Duration};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{PhysicalPosition, Window, WindowPosition};
use windows::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT, WPARAM},
    Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW, DWMWA_BORDER_COLOR,
        DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND, DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE,
    },
    UI::{
        HiDpi::{
            GetDpiForSystem, GetDpiForWindow, SetProcessDpiAwarenessContext,
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        },
        Input::KeyboardAndMouse::{GetAsyncKeyState, ReleaseCapture, VK_LBUTTON},
        WindowsAndMessaging::{
            GetCursorPos, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, SendMessageW,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
            HTCAPTION, HWND_NOTOPMOST, HWND_TOPMOST, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
            SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOW, WM_NCLBUTTONDOWN,
            WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
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
        let backdrop = DWMSBT_TRANSIENTWINDOW;
        let corner = DWMWCP_ROUND;
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

pub fn dpi_scale(window: &Window) -> Option<f32> {
    let hwnd = hwnd(window).ok()?;
    Some(unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0)
}

pub fn begin_window_drag(window: &Window) {
    if let Ok(hwnd) = hwnd(window) {
        unsafe {
            let _ = ReleaseCapture();
            SendMessageW(
                hwnd,
                WM_NCLBUTTONDOWN,
                Some(WPARAM(HTCAPTION as usize)),
                Some(LPARAM(0)),
            );
        }
    }
}

pub fn begin_horizontal_drag(window: &Window) {
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
    let fixed_y = rect.top.max(0);
    thread::spawn(move || {
        while unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0 {
            let mut point = POINT::default();
            if unsafe { GetCursorPos(&mut point) }.is_ok() {
                let x = start_window_x + point.x - start_cursor_x;
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
            }
            thread::sleep(Duration::from_millis(12));
        }
    });
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

pub fn cursor_inside(window: &Window) -> bool {
    let Ok(hwnd) = hwnd(window) else {
        return false;
    };
    let mut rect = RECT::default();
    let mut cursor = POINT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err()
        || unsafe { GetCursorPos(&mut cursor) }.is_err()
    {
        return false;
    }
    cursor.x >= rect.left && cursor.x < rect.right && cursor.y >= rect.top && cursor.y < rect.bottom
}

pub fn set_position(window: &Window, x: i32, y: i32) {
    window.set_position(WindowPosition::Physical(PhysicalPosition::new(x, y)));
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
}
