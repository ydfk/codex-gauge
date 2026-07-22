#[cfg(windows)]
pub fn apply_start_on_boot(_app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use winreg::{enums::*, RegKey};

    #[cfg(debug_assertions)]
    if enabled {
        return Err("开发模式不支持开机启动，请使用安装包版本".to_string());
    }

    let exe = std::env::current_exe().map_err(|_| "无法定位当前程序".to_string())?;
    let command = format!("\"{}\"", exe.display());
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_SET_VALUE,
        )
        .map_err(|_| "无法打开开机启动注册表项".to_string())?;

    if enabled {
        run_key
            .set_value("Codex Gauge", &command)
            .map_err(|_| "无法写入开机启动项".to_string())
    } else {
        match run_key.delete_value("Codex Gauge") {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("无法移除开机启动项".to_string()),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn apply_start_on_boot(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|_| "无法更新登录时启动设置".to_string())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn apply_start_on_boot(_app: &tauri::AppHandle, _enabled: bool) -> Result<(), String> {
    Ok(())
}
