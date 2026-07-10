use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::codex::{CodexUsageSnapshot, SnapshotStatus};
use crate::updater::UpdateCheckResult;
use crate::{AppState, WindowPinTarget};

const TRAY_ID: &str = "codex-gauge-tray";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, None)?;

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
            "toggle_top" => toggle_top_window(app),
            "detail" => toggle_named_window(app, "detail"),
            "settings" => toggle_named_window(app, "settings"),
            "refresh" => {
                let _ = app.emit("codex-gauge-refresh", ());
            }
            "always_on_top_main" => toggle_always_on_top(app, WindowPinTarget::Main),
            "always_on_top_top" => toggle_always_on_top(app, WindowPinTarget::Top),
            "lock_position" => toggle_lock_position(app),
            "update_check" => {
                let _ = app.emit("codex-gauge-check-update", ());
            }
            "update_install" => {
                let _ = app.emit("codex-gauge-install-update", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                bring_visible_windows_to_front(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_update_menu(app: &AppHandle, update: Option<&UpdateCheckResult>) {
    update_menu_with_status(app, update);
}

pub fn update_menu(app: &AppHandle) {
    let update = app
        .try_state::<AppState>()
        .and_then(|state| state.current_update_status());
    update_menu_with_status(app, update.as_ref());
}

fn update_menu_with_status(app: &AppHandle, update: Option<&UpdateCheckResult>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_menu(app, update) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu(
    app: &AppHandle,
    update: Option<&UpdateCheckResult>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let main_visible = window_visible(app, "main");
    let top_visible = window_visible(app, "top");
    let detail_visible = window_visible(app, "detail");
    let settings_visible = window_visible(app, "settings");
    let main_always_on_top = app
        .try_state::<AppState>()
        .map(|state| state.always_on_top_enabled(WindowPinTarget::Main))
        .unwrap_or(false);
    let top_always_on_top = app
        .try_state::<AppState>()
        .map(|state| state.always_on_top_enabled(WindowPinTarget::Top))
        .unwrap_or(false);
    let lock_position = app
        .try_state::<AppState>()
        .map(|state| state.lock_position_enabled())
        .unwrap_or(false);

    let toggle = MenuItem::with_id(
        app,
        "toggle",
        window_status_label("桌面浮窗", main_visible),
        true,
        None::<&str>,
    )?;
    let toggle_top = MenuItem::with_id(
        app,
        "toggle_top",
        window_status_label("顶部浮窗", top_visible),
        true,
        None::<&str>,
    )?;
    let detail = MenuItem::with_id(
        app,
        "detail",
        window_status_label("详细信息", detail_visible),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        window_status_label("设置", settings_visible),
        true,
        None::<&str>,
    )?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新用量", true, None::<&str>)?;
    let main_always_on_top_item = MenuItem::with_id(
        app,
        "always_on_top_main",
        if main_always_on_top {
            "✓ 桌面浮窗置顶"
        } else {
            "○ 桌面浮窗不置顶"
        },
        true,
        None::<&str>,
    )?;
    let top_always_on_top_item = MenuItem::with_id(
        app,
        "always_on_top_top",
        if top_always_on_top {
            "✓ 顶部浮窗置顶"
        } else {
            "○ 顶部浮窗不置顶"
        },
        true,
        None::<&str>,
    )?;
    let lock_position_item = MenuItem::with_id(
        app,
        "lock_position",
        if lock_position {
            "✓ 锁定桌面浮窗位置"
        } else {
            "○ 桌面浮窗位置可拖动"
        },
        true,
        None::<&str>,
    )?;
    let update_item = match update {
        Some(result) if result.available => MenuItem::with_id(
            app,
            "update_install",
            update_install_label(result),
            true,
            None::<&str>,
        )?,
        Some(result) => MenuItem::with_id(
            app,
            "update_check",
            update_check_label(result),
            true,
            None::<&str>,
        )?,
        None => MenuItem::with_id(
            app,
            "update_check",
            "更新：检查最新版本",
            true,
            None::<&str>,
        )?,
    };
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &toggle,
            &toggle_top,
            &detail,
            &settings,
            &refresh,
            &main_always_on_top_item,
            &top_always_on_top_item,
            &lock_position_item,
            &update_item,
            &quit,
        ],
    )
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
            persist_window_visibility(app, "main", false);
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            persist_window_visibility(app, "main", true);
        }
        update_menu(app);
    }
}

fn toggle_top_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("top") {
        let _ = window.set_size(crate::window::top_window_size(false));
        if window.is_visible().unwrap_or_default() {
            let _ = window.hide();
            persist_window_visibility(app, "top", false);
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            persist_window_visibility(app, "top", true);
        }
        update_menu(app);
    }
}

fn toggle_named_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if window.is_visible().unwrap_or_default() {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
        update_menu(app);
    }
}

fn toggle_always_on_top(app: &AppHandle, target: WindowPinTarget) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let enabled = state.toggle_always_on_top(target);
    let label = match target {
        WindowPinTarget::Main => "main",
        WindowPinTarget::Top => "top",
    };
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.set_always_on_top(enabled);
    }
    let _ = app.emit("codex-gauge-config-updated", ());
    update_menu(app);
}

fn toggle_lock_position(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    state.toggle_lock_position();
    let _ = app.emit("codex-gauge-config-updated", ());
    update_menu(app);
}

fn bring_visible_windows_to_front(app: &AppHandle) {
    for label in ["main", "top", "detail", "settings"] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        if window.is_visible().unwrap_or(false) {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn persist_window_visibility(app: &AppHandle, label: &str, visible: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if state.set_window_visibility_preference(label, visible) {
        let _ = app.emit("codex-gauge-config-updated", ());
    }
}

fn window_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

fn window_status_label(name: &str, visible: bool) -> String {
    if visible {
        format!("✓ {}：已打开", name)
    } else {
        format!("○ {}：已隐藏", name)
    }
}

fn default_tooltip() -> &'static str {
    "5h: 未知\n7d: 未知\n重置: 未知"
}

fn snapshot_tooltip(snapshot: &CodexUsageSnapshot) -> String {
    format!(
        "5h: 剩{} 用{} 重{}\n7d: 剩{} 用{} 重{}\n重置: {} · {}",
        percent(
            snapshot
                .primary_window
                .as_ref()
                .and_then(|window| window.remaining_percent)
        ),
        percent(
            snapshot
                .primary_window
                .as_ref()
                .and_then(|window| window.used_percent)
        ),
        compact_time(
            snapshot
                .primary_window
                .as_ref()
                .and_then(|window| window.reset_at)
        ),
        percent(
            snapshot
                .secondary_window
                .as_ref()
                .and_then(|window| window.remaining_percent)
        ),
        percent(
            snapshot
                .secondary_window
                .as_ref()
                .and_then(|window| window.used_percent)
        ),
        compact_time(
            snapshot
                .secondary_window
                .as_ref()
                .and_then(|window| window.reset_at)
        ),
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

fn update_check_label(result: &UpdateCheckResult) -> String {
    if result.message.contains("最新") {
        "更新：已是最新版".to_string()
    } else if result.message.contains("失败") || result.message.contains("无效") {
        "更新：检查失败".to_string()
    } else {
        "更新：重新检查".to_string()
    }
}

fn update_install_label(result: &UpdateCheckResult) -> String {
    result
        .version
        .as_ref()
        .map(|version| format!("安装更新 {}", version))
        .unwrap_or_else(|| "安装更新".to_string())
}
