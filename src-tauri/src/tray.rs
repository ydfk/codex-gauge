use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "打开 / 隐藏浮窗", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新", true, None::<&str>)?;
    let lock_position = MenuItem::with_id(app, "lock_position", "锁定位置", true, None::<&str>)?;
    let start_on_boot = MenuItem::with_id(app, "start_on_boot", "开机启动", true, None::<&str>)?;
    let auto_update = MenuItem::with_id(app, "auto_update", "自动更新", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let login = MenuItem::with_id(app, "login", "打开 Codex 登录", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &refresh,
            &lock_position,
            &start_on_boot,
            &auto_update,
            &settings,
            &login,
            &quit,
        ],
    )?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")).expect("tray icon")
    });

    TrayIconBuilder::with_id("codex-gauge-tray")
        .tooltip("Codex Gauge\n5h 未知 · 1w 未知 · R未知")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "toggle" | "settings" => toggle_window(app),
            "refresh" => {
                let _ = app.emit("codex-gauge-refresh", ());
            }
            "lock_position" => {
                let _ = app.emit("codex-gauge-toggle-lock", ());
            }
            "start_on_boot" => {
                let _ = app.emit("codex-gauge-toggle-start-on-boot", ());
            }
            "auto_update" => {
                let _ = app.emit("codex-gauge-toggle-auto-update", ());
            }
            "login" => {
                let _ = app.emit("codex-gauge-open-login", ());
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
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
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
