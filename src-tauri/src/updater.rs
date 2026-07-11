use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Error as UpdaterError, UpdaterExt};

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
        Err(message) => {
            state.record_update_event("check_update", "failed", "config_invalid");
            return Ok(record_update_result(&app, &state, failed_result(message)));
        }
    };
    let update = match updater.check().await {
        Ok(update) => update,
        Err(error) => {
            state.record_update_event("check_update", "failed", updater_error_category(&error));
            return Ok(record_update_result(
                &app,
                &state,
                failed_result("检查更新失败，请确认 GitHub Release 和签名配置"),
            ));
        }
    };

    let result = match update {
        Some(update) => {
            state.record_update_event("check_update", "success", "update_available");
            UpdateCheckResult {
                available: true,
                version: Some(update.version),
                message: "发现新版本，点击更新即可安装。".to_string(),
            }
        }
        None => {
            state.record_update_event("check_update", "success", "up_to_date");
            UpdateCheckResult {
                available: false,
                version: None,
                message: "当前已是最新版本。".to_string(),
            }
        }
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
        Err(message) => {
            state.record_update_event("install_update", "failed", "config_invalid");
            return Ok(record_update_result(&app, &state, failed_result(message)));
        }
    };
    let Some(update) = (match updater.check().await {
        Ok(update) => update,
        Err(error) => {
            state.record_update_event("install_update", "failed", updater_error_category(&error));
            return Ok(record_update_result(
                &app,
                &state,
                failed_result("检查更新失败，请确认 GitHub Release 和签名配置"),
            ));
        }
    }) else {
        state.record_update_event("install_update", "success", "up_to_date");
        let result = UpdateCheckResult {
            available: false,
            version: None,
            message: "当前已是最新版本。".to_string(),
        };
        return Ok(record_update_result(&app, &state, result));
    };

    let version = update.version.clone();
    if let Err(error) = update.download_and_install(|_, _| {}, || {}).await {
        let category = updater_error_category(&error);
        state.record_update_event("install_update", "failed", category);
        return Ok(record_update_result(
            &app,
            &state,
            failed_result(install_failure_message(category)),
        ));
    }

    state.record_update_event("install_update", "success", "installed");
    let result = UpdateCheckResult {
        available: false,
        version: Some(version),
        message: "更新已安装，应用即将按安装器要求重启。".to_string(),
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

fn updater_error_category(error: &UpdaterError) -> &'static str {
    match error {
        UpdaterError::Network(message) if message.contains("404") => "asset_not_found",
        UpdaterError::Network(_) | UpdaterError::Reqwest(_) => "network_error",
        UpdaterError::Minisign(_) | UpdaterError::Base64(_) | UpdaterError::SignatureUtf8(_) => {
            "signature_invalid"
        }
        UpdaterError::ReleaseNotFound
        | UpdaterError::Serialization(_)
        | UpdaterError::TargetNotFound(_)
        | UpdaterError::TargetsNotFound(_) => "manifest_invalid",
        UpdaterError::PackageInstallFailed
        | UpdaterError::AuthenticationFailed
        | UpdaterError::InvalidUpdaterFormat
        | UpdaterError::Io(_) => "install_failed",
        _ => "updater_error",
    }
}

fn install_failure_message(category: &str) -> &'static str {
    match category {
        "asset_not_found" => "更新包不存在，请确认 latest.json 与 Release 文件名一致",
        "signature_invalid" => "更新包签名验证失败，请确认 updater 公钥和签名密钥匹配",
        "network_error" => "更新包下载失败，请检查网络连接后重试",
        "manifest_invalid" => "更新清单无效，请检查 latest.json 配置",
        _ => "更新安装失败，请查看本地 update.log",
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
