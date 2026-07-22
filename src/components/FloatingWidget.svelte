<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { formatCompactDateTime, formatPercent, remainingTone, statusText } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let message = "";
  export let locked = false;
  export let onopen: () => void;
  export let onmenu: (event: MouseEvent) => void;

  $: fiveHourUnlimited = snapshot?.primaryWindowUnlimited ?? false;
  $: fiveHourWidth = fiveHourUnlimited
    ? "100%"
    : barWidth(snapshot?.primaryWindow?.remainingPercent);
  $: weeklyWidth = barWidth(snapshot?.secondaryWindow?.remainingPercent);
  $: fiveHourTone = fiveHourUnlimited
    ? "tone-full"
    : remainingTone(snapshot?.primaryWindow?.remainingPercent);
  $: weeklyTone = remainingTone(snapshot?.secondaryWindow?.remainingPercent);
  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";
  $: usedSummary = snapshot
    ? `${fiveHourUnlimited ? "5h 无限" : `5h已用 ${formatPercent(snapshot.primaryWindow?.usedPercent)}`} · 7d已用 ${formatPercent(snapshot.secondaryWindow?.usedPercent)}`
    : statusText(snapshot);
  $: headerStatus = message || usedSummary;

  let dragStart: { x: number; y: number; dragging: boolean } | null = null;
  let dragging = false;

  function barWidth(value: number | null | undefined) {
    return `${Math.max(0, Math.min(100, value ?? 0))}%`;
  }

  function handlePointerDown(event: PointerEvent) {
    if (locked || event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
    dragStart = { x: event.clientX, y: event.clientY, dragging: false };
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragStart || dragStart.dragging) return;
    const moved = Math.hypot(event.clientX - dragStart.x, event.clientY - dragStart.y);
    if (moved < 6) return;

    dragStart.dragging = true;
    dragging = true;
    void getCurrentWindow().startDragging().finally(() => {
      dragging = false;
      dragStart = null;
    });
  }

  function handlePointerUp() {
    dragStart = null;
    dragging = false;
  }

  function handleDoubleClick() {
    if (dragStart?.dragging) return;
    onopen();
  }

  function handleContextMenu(event: MouseEvent) {
    event.stopPropagation();
    onmenu(event);
  }
</script>

<section class="floating-widget">
  <div class="glass-refraction"></div>
  <div
    class="widget-body"
    class:dragging
    class:position-locked={locked}
    role="presentation"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
    oncontextmenu={handleContextMenu}
    ondblclick={handleDoubleClick}
  >
    <header class="widget-top">
      <div class="widget-identity">
        <span class="brand">Codex</span>
        <span class="status-copy" class:widget-notice={!!message} aria-live="polite">{headerStatus}</span>
      </div>
      <div class="widget-actions">
        <span class="credit-chip">重置 {resetCount}</span>
        <button class="detail-chip" type="button" onclick={onopen}>详情</button>
      </div>
    </header>

    <div class="usage-matrix">
      <div class="usage-line">
        <div class="usage-card-head">
          <span>5h</span>
          <strong>{fiveHourUnlimited ? "无限" : formatPercent(snapshot?.primaryWindow?.remainingPercent)}</strong>
        </div>
        <div class={`segmented-meter ${fiveHourTone}`}>
          <span style={`width: ${fiveHourWidth}`}></span>
        </div>
        <small>{fiveHourUnlimited ? "无需重置" : formatCompactDateTime(snapshot?.primaryWindow?.resetAt)}</small>
      </div>
      <div class="usage-line">
        <div class="usage-card-head">
          <span>7d</span>
          <strong>{formatPercent(snapshot?.secondaryWindow?.remainingPercent)}</strong>
        </div>
        <div class={`segmented-meter ${weeklyTone}`}>
          <span style={`width: ${weeklyWidth}`}></span>
        </div>
        <small>{formatCompactDateTime(snapshot?.secondaryWindow?.resetAt)}</small>
      </div>
    </div>
  </div>
</section>
