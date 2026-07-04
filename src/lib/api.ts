import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, CodexUsageSnapshot, ResetUsage, UpdateCheckResult } from "./types";

export function getSnapshot() {
  return invoke<CodexUsageSnapshot>("get_snapshot");
}

export function refreshSnapshot() {
  return invoke<CodexUsageSnapshot>("refresh_snapshot");
}

export function getConfig() {
  return invoke<AppConfig>("get_config");
}

export function saveConfig(config: AppConfig) {
  return invoke<AppConfig>("save_config", { config });
}

export function getResetStats() {
  return invoke<ResetUsage>("get_reset_stats");
}

export function checkUpdate() {
  return invoke<UpdateCheckResult>("check_update");
}

export function installUpdate() {
  return invoke<UpdateCheckResult>("install_update");
}

export function openCodexLogin() {
  return invoke<void>("open_codex_login");
}

export function showMainWindow() {
  return invoke<void>("show_main_window");
}

export function hideMainWindow() {
  return invoke<void>("hide_main_window");
}

export function setWindowMode(expanded: boolean) {
  return invoke<void>("set_window_mode", { expanded });
}

export function quitApp() {
  return invoke<void>("quit_app");
}
