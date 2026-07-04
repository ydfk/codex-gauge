use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use crate::{storage::AppConfig, tray, AppState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub available: bool,
    pub version: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckResult, String> {
    let config = state.config.lock().expect("config mutex").clone();
    let updater = match configured_updater(&app, &config) {
        Ok(updater) => updater,
        Err(message) => return Ok(record_update_result(&app, &state, failed_result(message))),
    };
    let update = match updater.check().await {
        Ok(update) => update,
        Err(_) => {
            return Ok(record_update_result(
                &app,
                &state,
                failed_result("检查更新失败，请确认 GitHub Release 和签名配置"),
            ));
        }
    };

    let result = match update {
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
    };
    Ok(record_update_result(&app, &state, result))
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckResult, String> {
    let config = state.config.lock().expect("config mutex").clone();
    let updater = match configured_updater(&app, &config) {
        Ok(updater) => updater,
        Err(message) => return Ok(record_update_result(&app, &state, failed_result(message))),
    };
    let Some(update) = (match updater.check().await {
        Ok(update) => update,
        Err(_) => {
            return Ok(record_update_result(
                &app,
                &state,
                failed_result("检查更新失败，请确认 GitHub Release 和签名配置"),
            ));
        }
    }) else {
        let result = UpdateCheckResult {
            available: false,
            version: None,
            message: "当前已是最新版本。".to_string(),
        };
        return Ok(record_update_result(&app, &state, result));
    };

    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| "下载或安装更新失败".to_string())?;

    let result = UpdateCheckResult {
        available: true,
        version: Some(version),
        message: "更新已安装，应用将按安装器要求重启或退出。".to_string(),
    };
    Ok(record_update_result(&app, &state, result))
}

fn configured_updater(
    app: &AppHandle,
    config: &AppConfig,
) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint = config.update.endpoint.trim();
    if endpoint.is_empty() {
        return app.updater().map_err(|_| "更新器配置不可用".to_string());
    }

    let url = reqwest::Url::parse(endpoint).map_err(|_| "更新地址格式无效".to_string())?;
    app.updater_builder()
        .endpoints(vec![url])
        .map_err(|_| "更新地址不受支持".to_string())?
        .build()
        .map_err(|_| "更新器配置不可用".to_string())
}

fn failed_result(message: impl Into<String>) -> UpdateCheckResult {
    UpdateCheckResult {
        available: false,
        version: None,
        message: message.into(),
    }
}

fn record_update_result(
    app: &AppHandle,
    state: &State<'_, AppState>,
    result: UpdateCheckResult,
) -> UpdateCheckResult {
    state.set_update_status(result.clone());
    tray::update_update_menu(app, Some(&result));
    result
}
