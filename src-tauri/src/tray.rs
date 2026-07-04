use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::codex::{CodexUsageSnapshot, SnapshotStatus};

const TRAY_ID: &str = "codex-gauge-tray";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "打开/隐藏浮窗", true, None::<&str>)?;
    let always_on_top = MenuItem::with_id(
        app,
        "always_on_top",
        "固定/取消固定在最上层",
        true,
        None::<&str>,
    )?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&toggle, &always_on_top, &refresh, &settings, &quit])?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")).expect("tray icon")
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(default_tooltip())
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "toggle" => toggle_window(app),
            "always_on_top" => {
                let _ = app.emit("codex-gauge-toggle-always-on-top", ());
            }
            "refresh" => {
                let _ = app.emit("codex-gauge-refresh", ());
            }
            "settings" => {
                open_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = tray.app_handle();
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tooltip(app: &AppHandle, snapshot: &CodexUsageSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(snapshot_tooltip(snapshot)));
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or_default() {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_size(crate::window::main_window_size(true));
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("codex-gauge-open-settings", ());
    }
}

fn default_tooltip() -> &'static str {
    "Codex Gauge\n5h 未知 · 7d 未知\n重置次数: 未知"
}

fn snapshot_tooltip(snapshot: &CodexUsageSnapshot) -> String {
    format!(
        "Codex Gauge\n5h 剩余 {} · 已用 {} · 重置 {}\n7d 剩余 {} · 已用 {} · 重置 {}\n重置次数: {} · {}",
        percent(snapshot.primary_window.as_ref().and_then(|window| window.remaining_percent)),
        percent(snapshot.primary_window.as_ref().and_then(|window| window.used_percent)),
        compact_time(snapshot.primary_window.as_ref().and_then(|window| window.reset_at)),
        percent(snapshot.secondary_window.as_ref().and_then(|window| window.remaining_percent)),
        percent(snapshot.secondary_window.as_ref().and_then(|window| window.used_percent)),
        compact_time(snapshot.secondary_window.as_ref().and_then(|window| window.reset_at)),
        snapshot
            .credits
            .as_ref()
            .and_then(|credits| credits.available_count.or(credits.reset_credits))
            .map(|count| count.to_string())
            .unwrap_or_else(|| "未知".to_string()),
        status_text(&snapshot.status),
    )
}

fn percent(value: Option<f64>) -> String {
    value
        .map(|item| format!("{}%", item.round()))
        .unwrap_or_else(|| "未知".to_string())
}

fn compact_time(value: Option<i64>) -> String {
    value
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "未知".to_string())
}

fn status_text(status: &SnapshotStatus) -> &'static str {
    match status {
        SnapshotStatus::Ok => "正常",
        SnapshotStatus::NotLoggedIn => "未登录",
        SnapshotStatus::InvalidAuth => "凭据失效",
        SnapshotStatus::RequestFailed => "查询失败",
    }
}
