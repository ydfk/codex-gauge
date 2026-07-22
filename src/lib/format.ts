import type { CodexUsageSnapshot, UsageWindow } from "./types";

export function formatPercent(value: number | null | undefined) {
  return value == null ? "未知" : `${Math.round(value)}%`;
}

export function remainingTone(value: number | null | undefined) {
  if (value == null) return "tone-muted";
  if (value <= 5) return "tone-empty";
  if (value <= 15) return "tone-critical";
  if (value <= 30) return "tone-low";
  if (value <= 50) return "tone-mid";
  if (value <= 70) return "tone-good";
  return "tone-full";
}

export function formatReset(unixSeconds: number | null | undefined) {
  if (unixSeconds == null) return "未知";
  const remaining = Math.max(0, Math.floor(unixSeconds - Date.now() / 1000));
  const days = Math.floor(remaining / 86_400);
  const hours = Math.floor((remaining % 86_400) / 3_600);
  const minutes = Math.floor((remaining % 3_600) / 60);

  if (days > 0) return `${days}d ${String(hours).padStart(2, "0")}h`;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

export function formatCompactDateTime(unixSeconds: number | null | undefined) {
  if (unixSeconds == null) return "未知";
  const parts = new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(new Date(unixSeconds * 1000));
  const map = Object.fromEntries(parts.map((part) => [part.type, part.value]));
  return `${map.month}-${map.day} ${map.hour}:${map.minute}`;
}

export function formatTokens(value: number | null | undefined) {
  if (value == null) return "未知";
  if (value >= 1_000_000) return `${trim(value / 1_000_000)}m`;
  if (value >= 1_000) return `${trim(value / 1_000)}k`;
  return String(value);
}

export function formatFullNumber(value: number | null | undefined) {
  return value == null ? "未知" : new Intl.NumberFormat("zh-CN").format(value);
}

export function formatDateTime(unixSeconds: number | null | undefined) {
  if (unixSeconds == null) return "未知";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(unixSeconds * 1000));
}

export function formatLastUpdated(unixSeconds: number | null | undefined) {
  if (unixSeconds == null) return "未知";
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(unixSeconds * 1000));
}

export function statusText(snapshot: CodexUsageSnapshot | null) {
  if (!snapshot) return "加载中";
  if (snapshot.status === "not_logged_in") return "未检测到 Codex 登录状态";
  if (snapshot.status === "invalid_auth") return "凭据失效或授权头无效";
  if (snapshot.status === "request_failed") return "Codex 用量查询失败";
  return "正常";
}

export function sourceText(snapshot: CodexUsageSnapshot | null) {
  if (!snapshot) return "加载中";
  if (snapshot.source === "auth-json") return "AuthJson";
  if (snapshot.source === "app-server") return "App Server";
  return "Session Log";
}

export function windowTitle(window: UsageWindow | null | undefined) {
  if (!window) return "未知窗口";
  if (window.name === "5h") return "5小时";
  return "7d";
}

function trim(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}
