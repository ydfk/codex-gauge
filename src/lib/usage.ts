import type { UsageWindow } from "./types";

export function ringValue(window: UsageWindow | null) {
  return Math.max(0, Math.min(100, window?.usedPercent ?? 0));
}

export function ringTone(value: number | null | undefined) {
  if (value == null) return "muted";
  if (value >= 95) return "danger";
  if (value >= 85) return "warning";
  if (value >= 60) return "notice";
  return "ok";
}
