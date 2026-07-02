mod autostart;
mod codex;
mod storage;
mod tray;
mod updater;
mod window;

use std::sync::Mutex;

use codex::{refresh_codex_snapshot, CodexGaugeSnapshot, ResetStats};
use storage::{AppConfig, AppStorage, StateDocument};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    storage: AppStorage,
    config: Mutex<AppConfig>,
    state_doc: Mutex<StateDocument>,
    snapshot: Mutex<Option<CodexGaugeSnapshot>>,
}

impl AppState {
    fn new(storage: AppStorage) -> Self {
        let config = storage.load_config();
        let state_doc = storage.load_state();

        Self {
            storage,
            config: Mutex::new(config),
            state_doc: Mutex::new(state_doc),
            snapshot: Mutex::new(None),
        }
    }

    fn refresh_snapshot(&self) -> CodexGaugeSnapshot {
        let config = self.config.lock().expect("config mutex").clone();
        let mut state_doc = self.state_doc.lock().expect("state mutex");
        let snapshot = refresh_codex_snapshot(&config, &mut state_doc);

        self.storage.save_state(&state_doc);
        self.storage.record_usage_snapshot(&snapshot);
        *self.snapshot.lock().expect("snapshot mutex") = Some(snapshot.clone());

        snapshot
    }

    fn save_window_position(&self, x: i32, y: i32) {
        let mut config = self.config.lock().expect("config mutex");
        if config.general.lock_position {
            return;
        }

        config.window.x = Some(x);
        config.window.y = Some(y);
        self.storage.save_config(&config);
    }
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> CodexGaugeSnapshot {
    if let Some(snapshot) = state.snapshot.lock().expect("snapshot mutex").clone() {
        return snapshot;
    }

    state.refresh_snapshot()
}

#[tauri::command]
fn refresh_snapshot(state: State<'_, AppState>) -> CodexGaugeSnapshot {
    state.refresh_snapshot()
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.lock().expect("config mutex").clone()
}

#[tauri::command]
fn save_config(
    config: AppConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<AppConfig, String> {
    {
        let mut current = state.config.lock().expect("config mutex");
        *current = config.clone();
        state.storage.save_config(&current);
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(config.general.always_on_top);
    }
    let _ = autostart::apply_start_on_boot(config.general.start_on_boot);

    Ok(config)
}

#[tauri::command]
fn get_reset_stats(state: State<'_, AppState>) -> ResetStats {
    state
        .state_doc
        .lock()
        .expect("state mutex")
        .reset_stats
        .clone()
}

#[tauri::command]
fn open_codex_login(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().expect("config mutex").clone();
    std::process::Command::new(config.codex.command)
        .arg("login")
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("无法启动 Codex 登录：{}", safe_error_kind(&err)))
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())
}

#[tauri::command]
fn hide_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window.hide().map_err(|err| err.to_string())
}

#[tauri::command]
fn set_window_mode(expanded: bool, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;

    window
        .set_size(crate::window::main_window_size(expanded))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn safe_error_kind(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "codex_not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "app_server_error",
    }
}

pub fn run() {
    let storage = AppStorage::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new(storage))
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            refresh_snapshot,
            get_config,
            save_config,
            get_reset_stats,
            updater::check_update,
            updater::install_update,
            open_codex_login,
            show_main_window,
            hide_main_window,
            set_window_mode,
            quit_app
        ])
        .setup(|app| {
            window::setup_main_window(app.handle())?;
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Codex Gauge");
}
