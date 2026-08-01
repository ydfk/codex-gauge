use std::thread;

use crate::updater;

use super::{lock, quit, Backend, UiBridge};

pub(super) fn start_update_check(bridge: UiBridge, backend: Backend) {
    bridge.set_update_state("正在检查更新…", "检查更新", false);
    thread::spawn(move || {
        let endpoint = lock(&backend.config).update.endpoint.clone();
        let result = updater::check(&endpoint);
        let (message, label, info) = match result {
            Ok(Some(info)) => (
                format!("发现新版本 v{}，点击安装后自动更新", info.version),
                format!("安装 v{}", info.version),
                Some(info),
            ),
            Ok(None) => ("已是最新版".to_string(), "检查更新".to_string(), None),
            Err(error) => {
                backend
                    .storage
                    .record_update_event("check", "failed", &error.to_string());
                ("检查更新失败".to_string(), "检查更新".to_string(), None)
            }
        };
        *lock(&backend.update) = info.clone();
        let _ = slint::invoke_from_event_loop(move || {
            bridge.set_update_state(&message, &label, info.is_some());
        });
    });
}

pub(super) fn start_update_install(bridge: UiBridge, backend: Backend) {
    let Some(update) = lock(&backend.update).clone() else {
        bridge.set_update_message("请先检查更新");
        return;
    };
    bridge.set_update_message("正在下载并验证更新…");
    thread::spawn(move || {
        let public_key = lock(&backend.config).update.public_key.clone();
        match updater::download_and_launch(&update, &public_key) {
            Ok(()) => {
                backend
                    .storage
                    .record_update_event("install", "started", "ok");
                let _ = slint::invoke_from_event_loop(move || quit(&backend));
            }
            Err(error) => {
                backend
                    .storage
                    .record_update_event("install", "failed", &error.to_string());
                let message = match error {
                    updater::UpdateError::SignatureConfig => "未配置原生版更新公钥",
                    updater::UpdateError::SignatureInvalid => "更新包签名验证失败",
                    updater::UpdateError::NoWindowsAsset => "Release 中没有 Windows x64 更新包",
                    _ => "下载或安装更新失败，请查看 update.log",
                };
                let _ = slint::invoke_from_event_loop(move || bridge.set_update_message(message));
            }
        }
    });
}
