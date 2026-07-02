import type { CodexGaugeSnapshot, UsageWindow } from "./types";

export function formatPercent(value: number | null | undefined) {
  return value == null ? "未知" : `${Math.round(value)}%`;
}

export function formatReset(value: string | null | undefined) {
  return value || "未知";
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

export function statusText(snapshot: CodexGaugeSnapshot | null) {
  if (!snapshot) return "加载中";
  if (snapshot.status === "not_logged_in") return "未登录";
  if (snapshot.status === "codex_not_found") return "未检测到 Codex 服务";
  if (snapshot.status === "app_server_error") return "Codex App Server 异常";
  if (snapshot.status === "partial") return "部分数据";
  return "在线";
}

export function usageLevel(snapshot: CodexGaugeSnapshot | null) {
  if (!snapshot || snapshot.status !== "ok") return "muted";
  const values = [snapshot.fiveHour?.usedPercent, snapshot.weekly?.usedPercent].filter(
    (value): value is number => typeof value === "number",
  );
  const max = values.length ? Math.max(...values) : 0;
  if (max >= 95) return "danger";
  if (max >= 85) return "warning";
  if (max >= 60) return "notice";
  return "ok";
}

export function windowTitle(window: UsageWindow | null | undefined) {
  if (!window) return "未知窗口";
  if (window.label === "5h") return "5小时";
  if (window.label === "1w") return "一周";
  return "其他";
}

function trim(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}
