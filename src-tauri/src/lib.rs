mod autostart;
mod codex;
mod storage;
mod tray;
mod updater;
mod window;

use std::{collections::HashMap, sync::Mutex};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use codex::{refresh_codex_snapshot, CodexUsageSnapshot, ResetStats, SnapshotStatus};
use storage::{AppConfig, AppStorage, StateDocument};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};

const SNAPSHOT_CACHE_SECONDS: i64 = 120;

pub struct AppState {
    storage: AppStorage,
    config: Mutex<AppConfig>,
    state_doc: Mutex<StateDocument>,
    snapshot: Mutex<Option<CodexUsageSnapshot>>,
    refresh_cache: Mutex<RefreshCache>,
    update_status: Mutex<Option<updater::UpdateCheckResult>>,
    oled_moves: Mutex<HashMap<String, (i32, i32)>>,
}

#[derive(Debug, Default)]
struct RefreshCache {
    failure_count: u32,
    retry_after_at: Option<i64>,
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
            refresh_cache: Mutex::new(RefreshCache::default()),
            update_status: Mutex::new(None),
            oled_moves: Mutex::new(HashMap::new()),
        }
    }

    fn get_snapshot_cached(&self, force: bool) -> CodexUsageSnapshot {
        let now = chrono::Local::now().timestamp();
        if !force {
            if let Some(snapshot) = self.snapshot.lock().expect("snapshot mutex").clone() {
                if now - snapshot.updated_at < SNAPSHOT_CACHE_SECONDS {
                    return snapshot;
                }
            }

            let retry_after_at = self
                .refresh_cache
                .lock()
                .expect("refresh cache mutex")
                .retry_after_at;
            if retry_after_at.is_some_and(|retry_at| retry_at > now) {
                if let Some(snapshot) = self.snapshot.lock().expect("snapshot mutex").clone() {
                    return snapshot;
                }
            }
        }

        self.refresh_snapshot_now()
    }

    fn refresh_snapshot_now(&self) -> CodexUsageSnapshot {
        let config = self.config.lock().expect("config mutex").clone();
        let mut state_doc = self.state_doc.lock().expect("state mutex");
        let previous = self.snapshot.lock().expect("snapshot mutex").clone();
        let snapshot =
            merge_failed_snapshot(refresh_codex_snapshot(&config, &mut state_doc), previous);

        self.update_refresh_cache(&snapshot);
        state_doc.last_snapshot = Some(snapshot.clone());
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

    pub(crate) fn save_top_window_position(&self, x: i32) {
        let mut config = self.config.lock().expect("config mutex");
        if config.general.top_lock_position {
            return;
        }
        config.window.top_x = Some(x);
        self.storage.save_config(&config);
    }

    fn mark_oled_move(&self, label: &str, x: i32, y: i32) {
        self.oled_moves
            .lock()
            .expect("oled moves mutex")
            .insert(label.to_string(), (x, y));
    }

    fn clear_oled_move(&self, label: &str) {
        self.oled_moves
            .lock()
            .expect("oled moves mutex")
            .remove(label);
    }

    pub(crate) fn is_oled_move(&self, label: &str, x: i32, y: i32) -> bool {
        let mut moves = self.oled_moves.lock().expect("oled moves mutex");
        if moves.get(label) != Some(&(x, y)) {
            return false;
        }
        moves.remove(label);
        true
    }

    fn update_refresh_cache(&self, snapshot: &CodexUsageSnapshot) {
        let mut cache = self.refresh_cache.lock().expect("refresh cache mutex");
        if snapshot.status == SnapshotStatus::Ok {
            cache.failure_count = 0;
            cache.retry_after_at = None;
            return;
        }

        cache.failure_count = cache.failure_count.saturating_add(1);
        let delay = match cache.failure_count {
            1 => 30,
            2 => 60,
            3 => 120,
            _ => 300,
        };
        cache.retry_after_at = Some(chrono::Local::now().timestamp() + delay);
    }

    fn set_update_status(&self, status: updater::UpdateCheckResult) {
        *self.update_status.lock().expect("update status mutex") = Some(status);
    }

    pub(crate) fn current_update_status(&self) -> Option<updater::UpdateCheckResult> {
        self.update_status
            .lock()
            .expect("update status mutex")
            .clone()
    }

    pub(crate) fn start_on_boot_enabled(&self) -> bool {
        self.config
            .lock()
            .expect("config mutex")
            .general
            .start_on_boot
    }

    #[cfg(debug_assertions)]
    fn disable_debug_start_on_boot(&self) {
        let mut config = self.config.lock().expect("config mutex");
        if !config.general.start_on_boot {
            return;
        }
        config.general.start_on_boot = false;
        self.storage.save_config(&config);
    }

    pub(crate) fn set_window_visibility_preference(&self, label: &str, visible: bool) -> bool {
        let mut config = self.config.lock().expect("config mutex");
        let preference = match label {
            "main" => &mut config.general.show_on_startup,
            "top" => &mut config.general.top_status_enabled,
            _ => return false,
        };
        if *preference == visible {
            return false;
        }

        *preference = visible;
        self.storage.save_config(&config);
        true
    }

    pub(crate) fn always_on_top_enabled(&self, target: WindowPinTarget) -> bool {
        let config = self.config.lock().expect("config mutex");
        match target {
            WindowPinTarget::Main => config.general.main_always_on_top,
            WindowPinTarget::Top => config.general.top_always_on_top,
        }
    }

    pub(crate) fn lock_position_enabled(&self, target: WindowLockTarget) -> bool {
        let config = self.config.lock().expect("config mutex");
        match target {
            WindowLockTarget::Main => config.general.lock_position,
            WindowLockTarget::Top => config.general.top_lock_position,
        }
    }

    pub(crate) fn toggle_lock_position(&self, target: WindowLockTarget) -> bool {
        let mut config = self.config.lock().expect("config mutex");
        let enabled = match target {
            WindowLockTarget::Main => {
                config.general.lock_position = !config.general.lock_position;
                config.general.lock_position
            }
            WindowLockTarget::Top => {
                config.general.top_lock_position = !config.general.top_lock_position;
                config.general.top_lock_position
            }
        };
        self.storage.save_config(&config);
        enabled
    }

    pub(crate) fn toggle_always_on_top(&self, target: WindowPinTarget) -> bool {
        let mut config = self.config.lock().expect("config mutex");
        let enabled = match target {
            WindowPinTarget::Main => {
                config.general.main_always_on_top = !config.general.main_always_on_top;
                config.general.main_always_on_top
            }
            WindowPinTarget::Top => {
                config.general.top_always_on_top = !config.general.top_always_on_top;
                config.general.top_always_on_top
            }
        };
        self.storage.save_config(&config);
        enabled
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WindowPinTarget {
    Main,
    Top,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WindowLockTarget {
    Main,
    Top,
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>, app: AppHandle) -> CodexUsageSnapshot {
    let snapshot = state.get_snapshot_cached(false);
    tray::update_tooltip(&app, &snapshot);
    snapshot
}

#[tauri::command]
fn refresh_snapshot(state: State<'_, AppState>, app: AppHandle) -> CodexUsageSnapshot {
    let snapshot = state.get_snapshot_cached(true);
    tray::update_tooltip(&app, &snapshot);
    snapshot
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
    autostart::apply_start_on_boot(config.general.start_on_boot)?;
    {
        let mut current = state.config.lock().expect("config mutex");
        *current = config.clone();
        state.storage.save_config(&current);
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(config.general.main_always_on_top);
        if config.general.show_on_startup {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }
    if let Some(window) = app.get_webview_window("top") {
        let _ = window.set_always_on_top(config.general.top_always_on_top);
        if config.general.top_status_enabled {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }
    let _ = app.emit("codex-gauge-config-updated", ());
    tray::update_menu(&app);

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
    let mut command = std::process::Command::new(config.codex.command);
    command.arg("login");
    hide_windows_console(&mut command);

    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("无法启动 Codex 登录：{}", safe_error_kind(&err)))
}

#[cfg(windows)]
fn hide_windows_console(command: &mut std::process::Command) {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_windows_console(_command: &mut std::process::Command) {}

#[tauri::command]
fn show_main_window(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;
    persist_window_visibility(&state, &app, "main", true);
    tray::update_menu(&app);
    Ok(())
}

#[tauri::command]
fn hide_main_window(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window.hide().map_err(|err| err.to_string())?;
    persist_window_visibility(&state, &app, "main", false);
    tray::update_menu(&app);
    Ok(())
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
fn set_top_context_menu(open: bool, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("top")
        .ok_or_else(|| "顶部浮条不存在".to_string())?;

    window
        .set_size(crate::window::top_window_size(open))
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn move_window_for_oled(
    label: String,
    offset_x: i32,
    offset_y: i32,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if !matches!(label.as_str(), "main" | "top") {
        return Err("窗口不支持防烧屏位移".to_string());
    }

    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "窗口不存在".to_string())?;
    let position = window.outer_position().map_err(|err| err.to_string())?;
    let next = PhysicalPosition::new(position.x + offset_x, position.y + offset_y);
    state.mark_oled_move(&label, next.x, next.y);

    if let Err(err) = window.set_position(next) {
        state.clear_oled_move(&label);
        return Err(err.to_string());
    }
    Ok(())
}

#[tauri::command]
fn show_window(label: String, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "窗口不存在".to_string())?;
    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;
    persist_window_visibility(&state, &app, &label, true);
    tray::update_menu(&app);
    Ok(())
}

#[tauri::command]
fn hide_window(label: String, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    if label == "top" {
        let _ = set_top_context_menu(false, app.clone());
    }
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "窗口不存在".to_string())?;
    window.hide().map_err(|err| err.to_string())?;
    persist_window_visibility(&state, &app, &label, false);
    tray::update_menu(&app);
    Ok(())
}

#[tauri::command]
fn toggle_window_visible(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<bool, String> {
    if label == "top" {
        let _ = set_top_context_menu(false, app.clone());
    }
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "窗口不存在".to_string())?;

    let visible = window.is_visible().map_err(|err| err.to_string())?;
    if visible {
        window.hide().map_err(|err| err.to_string())?;
        persist_window_visibility(&state, &app, &label, false);
        tray::update_menu(&app);
        return Ok(false);
    }

    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;
    persist_window_visibility(&state, &app, &label, true);
    tray::update_menu(&app);
    Ok(true)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn persist_window_visibility(state: &AppState, app: &AppHandle, label: &str, visible: bool) {
    if state.set_window_visibility_preference(label, visible) {
        let _ = app.emit("codex-gauge-config-updated", ());
    }
}

fn safe_error_kind(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "codex_not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "app_server_error",
    }
}

fn merge_failed_snapshot(
    snapshot: CodexUsageSnapshot,
    previous: Option<CodexUsageSnapshot>,
) -> CodexUsageSnapshot {
    if snapshot.status == SnapshotStatus::Ok || !has_usage_data(previous.as_ref()) {
        return snapshot;
    }

    let mut merged = previous.expect("checked by has_usage_data");
    merged.source = snapshot.source;
    merged.status = snapshot.status;
    merged.updated_at = snapshot.updated_at;
    merged.rate_limit_reached_type = snapshot.rate_limit_reached_type;
    if snapshot.credits.is_some() {
        merged.credits = snapshot.credits;
    }
    if snapshot.plan_type.is_some() {
        merged.plan_type = snapshot.plan_type;
    }
    merged
}

fn has_usage_data(snapshot: Option<&CodexUsageSnapshot>) -> bool {
    snapshot
        .map(|item| item.primary_window.is_some() || item.secondary_window.is_some())
        .unwrap_or(false)
}

pub fn run() {
    let storage = AppStorage::new();
    let state = AppState::new(storage);

    #[cfg(debug_assertions)]
    state.disable_debug_start_on_boot();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
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
            set_top_context_menu,
            move_window_for_oled,
            show_window,
            hide_window,
            toggle_window_visible,
            quit_app
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let _ = autostart::apply_start_on_boot(state.start_on_boot_enabled());
            window::setup_main_window(app.handle())?;
            window::setup_top_window(app.handle())?;
            window::setup_panel_windows(app.handle());
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Codex Gauge");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_main_and_top_visibility_preferences() {
        let temp = tempfile::tempdir().expect("temp dir");
        let state = AppState::new(AppStorage::with_root(temp.path().to_path_buf()));

        assert!(state.set_window_visibility_preference("main", false));
        assert!(state.set_window_visibility_preference("top", false));
        state.save_top_window_position(256);
        assert!(state.toggle_lock_position(WindowLockTarget::Main));
        assert!(state.toggle_lock_position(WindowLockTarget::Top));
        state.save_top_window_position(512);

        let config = state.storage.load_config();
        assert!(!config.general.show_on_startup);
        assert!(!config.general.top_status_enabled);
        assert!(config.general.lock_position);
        assert!(config.general.top_lock_position);
        assert_eq!(config.window.top_x, Some(256));
    }
}
