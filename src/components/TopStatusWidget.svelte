<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { formatPercent, statusText, usageLevel } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let busy = false;
  export let onmenu: (event: MouseEvent) => void;

  $: level = usageLevel(snapshot);
  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";

  let dragStart: { x: number; y: number; dragging: boolean } | null = null;

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    dragStart = { x: event.clientX, y: event.clientY, dragging: false };
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragStart || dragStart.dragging) return;
    const moved = Math.hypot(event.clientX - dragStart.x, event.clientY - dragStart.y);
    if (moved < 5) return;

    dragStart.dragging = true;
    void getCurrentWindow().startDragging();
  }

  function handlePointerUp() {
    dragStart = null;
  }
</script>

<section
  class={`top-status-widget level-${level}`}
  role="presentation"
  title={statusText(snapshot)}
  oncontextmenu={onmenu}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
>
  <span class={`status-dot ${busy ? "pulse" : ""}`}></span>
  <strong>5h {formatPercent(snapshot?.primaryWindow?.remainingPercent)}</strong>
  <span>7d {formatPercent(snapshot?.secondaryWindow?.remainingPercent)}</span>
  <em>重置 {resetCount}</em>
</section>
