mod indicator;

#[cfg(not(target_os = "macos"))]
use tauri::menu::Submenu;
#[cfg(target_os = "macos")]
use tauri::tray::{MouseButton, MouseButtonState};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::codex::{CodexUsageSnapshot, SnapshotStatus};
use crate::updater::UpdateCheckResult;
use crate::{AppState, WindowLockTarget, WindowPinTarget};
use indicator::{base_tray_icon, tooltip_with_update, update_indicator};

const TRAY_ID: &str = "codex-gauge-tray";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, None)?;
    let icon = base_tray_icon(app);

    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(default_tooltip())
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false);
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(false).title("5h — · 7d —");

    builder
        .on_menu_event(move |app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            #[cfg(target_os = "macos")]
            if let TrayIconEvent::Click {
                position,
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_menubar_window(tray.app_handle(), Some((position.x, position.y)));
                return;
            }

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
    update_indicator(app, &tray, update);
    update_menubar_title(app);
}

#[cfg(target_os = "macos")]
fn build_menu(
    app: &AppHandle,
    _update: Option<&UpdateCheckResult>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "toggle", "打开详细信息", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新用量", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Codex Gauge", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &refresh, &settings, &separator, &quit])
}

#[cfg(not(target_os = "macos"))]
fn build_menu(
    app: &AppHandle,
    update: Option<&UpdateCheckResult>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let current_version = format!("v{}", app.package_info().version);
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
    let main_lock_position = app
        .try_state::<AppState>()
        .map(|state| state.lock_position_enabled(WindowLockTarget::Main))
        .unwrap_or(false);
    let top_lock_position = app
        .try_state::<AppState>()
        .map(|state| state.lock_position_enabled(WindowLockTarget::Top))
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
    let main_lock_position_item = MenuItem::with_id(
        app,
        "lock_position_main",
        if main_lock_position {
            "✓ 锁定桌面浮窗位置"
        } else {
            "○ 桌面浮窗位置可拖动"
        },
        true,
        None::<&str>,
    )?;
    let top_lock_position_item = MenuItem::with_id(
        app,
        "lock_position_top",
        if top_lock_position {
            "✓ 锁定顶部浮条位置"
        } else {
            "○ 顶部浮条位置可拖动"
        },
        true,
        None::<&str>,
    )?;
    let update_item = match update {
        Some(result) if result.available => MenuItem::with_id(
            app,
            "update_install",
            update_install_label(&current_version, result),
            true,
            None::<&str>,
        )?,
        Some(result) => MenuItem::with_id(
            app,
            "update_check",
            update_check_label(&current_version, result),
            true,
            None::<&str>,
        )?,
        None => MenuItem::with_id(
            app,
            "update_check",
            format!("更新：当前 {}（检查更新）", current_version),
            true,
            None::<&str>,
        )?,
    };
    let controls_separator = PredefinedMenuItem::separator(app)?;
    let update_separator = PredefinedMenuItem::separator(app)?;
    let quit_separator = PredefinedMenuItem::separator(app)?;
    let window_controls = Submenu::with_items(
        app,
        "浮窗控制",
        true,
        &[
            &toggle,
            &toggle_top,
            &controls_separator,
            &main_always_on_top_item,
            &top_always_on_top_item,
            &main_lock_position_item,
            &top_lock_position_item,
        ],
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &refresh,
            &window_controls,
            &detail,
            &settings,
            &update_separator,
            &update_item,
            &quit_separator,
            &quit,
        ],
    )
}

pub fn update_tooltip(app: &AppHandle, snapshot: &CodexUsageSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let update = app
            .try_state::<AppState>()
            .and_then(|state| state.current_update_status());
        let tooltip = tooltip_with_update(snapshot_tooltip(snapshot), update.as_ref());
        let _ = tray.set_tooltip(Some(tooltip));
    }
    update_menubar_title(app);
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "toggle" => toggle_primary_window(app),
        "toggle_top" => toggle_top_window(app),
        "detail" => toggle_named_window(app, "detail"),
        "settings" => open_settings(app),
        "refresh" => {
            let _ = app.emit("codex-gauge-refresh", ());
        }
        "always_on_top_main" => toggle_always_on_top(app, WindowPinTarget::Main),
        "always_on_top_top" => toggle_always_on_top(app, WindowPinTarget::Top),
        "lock_position_main" => toggle_lock_position(app, WindowLockTarget::Main),
        "lock_position_top" => toggle_lock_position(app, WindowLockTarget::Top),
        "update_check" => {
            let _ = app.emit("codex-gauge-check-update", ());
        }
        "update_install" => {
            let _ = app.emit("codex-gauge-install-update", ());
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn toggle_primary_window(app: &AppHandle) {
    toggle_menubar_window(app, None);
}

#[cfg(not(target_os = "macos"))]
fn toggle_primary_window(app: &AppHandle) {
    toggle_window(app);
}

#[cfg(target_os = "macos")]
fn open_settings(app: &AppHandle) {
    crate::window::show_menubar_window(app, None);
    let _ = app.emit("codex-gauge-open-macos-settings", ());
}

#[cfg(not(target_os = "macos"))]
fn open_settings(app: &AppHandle) {
    toggle_named_window(app, "settings");
}

#[cfg(target_os = "macos")]
fn toggle_menubar_window(app: &AppHandle, anchor: Option<(f64, f64)>) {
    let Some(window) = app.get_webview_window("menubar") else {
        return;
    };
    if window.is_visible().unwrap_or_default() {
        let _ = window.hide();
    } else {
        if app
            .try_state::<AppState>()
            .is_some_and(|state| state.should_suppress_menubar_reopen())
        {
            return;
        }
        let _ = app.emit("codex-gauge-open-macos-detail", ());
        crate::window::show_menubar_window(app, anchor);
    }
}

#[cfg(target_os = "macos")]
fn update_menubar_title(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mode = state
        .config
        .lock()
        .expect("config mutex")
        .macos
        .menu_bar_display
        .clone();
    let title = menu_bar_title(state.current_snapshot().as_ref(), &mode);
    let _ = tray.set_title(title.as_deref());
}

#[cfg(not(target_os = "macos"))]
fn update_menubar_title(_app: &AppHandle) {}

#[cfg(any(target_os = "macos", test))]
fn menu_bar_title(snapshot: Option<&CodexUsageSnapshot>, mode: &str) -> Option<String> {
    if mode == "iconOnly" {
        return None;
    }

    let weekly = snapshot
        .and_then(|snapshot| snapshot.secondary_window.as_ref())
        .and_then(|window| window.remaining_percent);
    if snapshot.is_some_and(|snapshot| snapshot.primary_window_unlimited) {
        return (mode == "fiveAndSeven").then(|| format!("7d {}", percent_compact(weekly)));
    }

    let five_hour = percent_compact(
        snapshot
            .and_then(|snapshot| snapshot.primary_window.as_ref())
            .and_then(|window| window.remaining_percent),
    );
    if mode != "fiveAndSeven" {
        return Some(format!("5h {}", five_hour));
    }

    Some(format!("5h {} · 7d {}", five_hour, percent_compact(weekly)))
}

#[cfg(any(target_os = "macos", test))]
fn percent_compact(value: Option<f64>) -> String {
    value
        .map(|item| format!("{}%", item.round()))
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(not(target_os = "macos"))]
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

fn toggle_lock_position(app: &AppHandle, target: WindowLockTarget) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    state.toggle_lock_position(target);
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

#[cfg(not(target_os = "macos"))]
fn window_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
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
    let five_hour = if snapshot.primary_window_unlimited {
        "5h: 无限".to_string()
    } else {
        format!(
            "5h: 剩{} 用{} 重{}",
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
        )
    };

    format!(
        "{}\n7d: 剩{} 用{} 重{}\n重置: {} · {}",
        five_hour,
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

#[cfg(not(target_os = "macos"))]
fn update_check_label(current_version: &str, result: &UpdateCheckResult) -> String {
    if result.message.contains("最新") {
        format!("更新：{}（已是最新版）", current_version)
    } else if result.message.contains("失败") || result.message.contains("无效") {
        format!("更新：{}（检查失败，点击重试）", current_version)
    } else {
        format!("更新：{}（重新检查）", current_version)
    }
}

#[cfg(not(target_os = "macos"))]
fn update_install_label(current_version: &str, result: &UpdateCheckResult) -> String {
    result
        .version
        .as_ref()
        .map(|version| format!("更新：{} → v{}（点击安装）", current_version, version))
        .unwrap_or_else(|| format!("更新：{}（点击安装）", current_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::{SnapshotSource, UsageWindow};

    fn snapshot(remaining: Option<f64>, weekly: Option<f64>) -> CodexUsageSnapshot {
        CodexUsageSnapshot {
            source: SnapshotSource::AppServer,
            status: SnapshotStatus::Ok,
            plan_type: None,
            primary_window: Some(UsageWindow {
                name: "5h".to_string(),
                used_percent: remaining.map(|value| 100.0 - value),
                remaining_percent: remaining,
                reset_at: None,
                window_duration_seconds: Some(18_000),
            }),
            primary_window_unlimited: false,
            secondary_window: Some(UsageWindow {
                name: "weekly".to_string(),
                used_percent: weekly.map(|value| 100.0 - value),
                remaining_percent: weekly,
                reset_at: None,
                window_duration_seconds: Some(604_800),
            }),
            credits: None,
            rate_limit_reached_type: None,
            updated_at: 0,
        }
    }

    #[test]
    fn formats_macos_menu_bar_modes() {
        let snapshot = snapshot(Some(72.4), Some(41.2));

        assert_eq!(
            menu_bar_title(Some(&snapshot), "fiveHour"),
            Some("5h 72%".to_string())
        );
        assert_eq!(
            menu_bar_title(Some(&snapshot), "fiveAndSeven"),
            Some("5h 72% · 7d 41%".to_string())
        );
        assert_eq!(menu_bar_title(Some(&snapshot), "iconOnly"), None);
    }

    #[test]
    fn hides_unlimited_five_hour_from_menu_bar() {
        let mut unlimited = snapshot(None, Some(80.0));
        unlimited.primary_window_unlimited = true;

        assert_eq!(
            menu_bar_title(Some(&unlimited), "fiveAndSeven"),
            Some("7d 80%".to_string())
        );
        assert_eq!(menu_bar_title(Some(&unlimited), "fiveHour"), None);
    }

    #[test]
    fn formats_unknown_menu_bar_values() {
        assert_eq!(menu_bar_title(None, "fiveHour"), Some("5h —".to_string()));
    }
}
