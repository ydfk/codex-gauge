use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub available: bool,
    pub version: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let updater = app.updater().map_err(|_| "更新器配置不可用".to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|_| "检查更新失败，请确认 GitHub Release 和签名配置".to_string())?;

    Ok(match update {
        Some(update) => UpdateCheckResult {
            available: true,
            version: Some(update.version),
            message: "发现新版本，可手动安装。".to_string(),
        },
        None => UpdateCheckResult {
            available: false,
            version: None,
            message: "当前已是最新版本。".to_string(),
        },
    })
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let updater = app.updater().map_err(|_| "更新器配置不可用".to_string())?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|_| "检查更新失败，请确认 GitHub Release 和签名配置".to_string())?
    else {
        return Ok(UpdateCheckResult {
            available: false,
            version: None,
            message: "当前已是最新版本。".to_string(),
        });
    };

    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| "下载或安装更新失败".to_string())?;

    Ok(UpdateCheckResult {
        available: true,
        version: Some(version),
        message: "更新已安装，应用将按安装器要求重启或退出。".to_string(),
    })
}
