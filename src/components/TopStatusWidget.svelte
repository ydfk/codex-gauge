<script lang="ts">
  import { PhysicalPosition } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { formatCompactDateTime, formatPercent } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let locked = false;
  export let detailsOpen = false;
  export let onmenu: (event: MouseEvent) => void;
  export let ondetail: () => void;
  export let onhoverchange: (open: boolean) => void;

  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";
  $: fiveHourUnlimited = snapshot?.primaryWindowUnlimited ?? false;
  $: fiveHourTone = fiveHourUnlimited
    ? "tone-full"
    : valueTone(snapshot?.primaryWindow?.remainingPercent);
  $: weeklyTone = valueTone(snapshot?.secondaryWindow?.remainingPercent);
  let horizontalDrag:
    | {
        pointerId: number;
        startScreenX: number;
        windowX: number;
        windowY: number;
        nextX: number;
        raf: number | null;
      }
    | null = null;
  let dragging = false;

  function valueTone(value: number | null | undefined) {
    if (value == null) return "tone-muted";
    if (value <= 5) return "tone-empty";
    if (value <= 15) return "tone-critical";
    if (value <= 30) return "tone-low";
    if (value <= 50) return "tone-mid";
    if (value <= 70) return "tone-good";
    return "tone-full";
  }

  async function handlePointerDown(event: PointerEvent) {
    if (locked || event.button !== 0) return;
    if (detailsOpen) onhoverchange(false);
    const target = event.currentTarget as HTMLElement;
    let position: PhysicalPosition;
    try {
      target.setPointerCapture(event.pointerId);
      position = await getCurrentWindow().outerPosition();
    } catch {
      dragging = false;
      horizontalDrag = null;
      return;
    }
    horizontalDrag = {
      pointerId: event.pointerId,
      startScreenX: event.screenX,
      windowX: position.x,
      windowY: position.y,
      nextX: position.x,
      raf: null,
    };
    dragging = true;
  }

  function handlePointerMove(event: PointerEvent) {
    if (!horizontalDrag) {
      showDetails();
      return;
    }
    horizontalDrag.nextX = Math.round(
      horizontalDrag.windowX + event.screenX - horizontalDrag.startScreenX,
    );
    if (horizontalDrag.raf != null) return;

    horizontalDrag.raf = window.requestAnimationFrame(() => {
      if (!horizontalDrag) return;
      horizontalDrag.raf = null;
      void getCurrentWindow().setPosition(
        new PhysicalPosition(horizontalDrag.nextX, horizontalDrag.windowY),
      );
    });
  }

  function handlePointerEnd(event: PointerEvent) {
    if (!horizontalDrag) return;
    if (horizontalDrag.raf != null) window.cancelAnimationFrame(horizontalDrag.raf);
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(horizontalDrag.pointerId);
    } catch {
      // 指针可能已被系统释放，保持拖拽状态收尾即可。
    }
    horizontalDrag = null;
    dragging = false;
  }

  function handleContextMenu(event: MouseEvent) {
    event.stopPropagation();
    onmenu(event);
  }

  function handleDoubleClick(event: MouseEvent) {
    if (event.button === 0 && !dragging) ondetail();
  }

  function showDetails() {
    if (!dragging && !detailsOpen) onhoverchange(true);
  }

  function hideDetails() {
    if (!dragging) onhoverchange(false);
  }
</script>

<section
  class="top-status-widget"
  class:dragging
  class:details-open={detailsOpen}
  class:position-locked={locked}
  role="presentation"
  oncontextmenu={handleContextMenu}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerEnd}
  onpointercancel={handlePointerEnd}
  ondblclick={handleDoubleClick}
  onmouseenter={showDetails}
  onmouseleave={hideDetails}
>
  <div class="top-summary">
    <strong class={fiveHourTone}>5h {fiveHourUnlimited ? "无限" : formatPercent(snapshot?.primaryWindow?.remainingPercent)}</strong>
    <span class={weeklyTone}>7d {formatPercent(snapshot?.secondaryWindow?.remainingPercent)}</span>
    <em>重置 {resetCount}</em>
  </div>

  {#if detailsOpen}
    <div class="top-hover-details" role="status">
      <div class="top-hover-row">
        <span>5h</span>
        <strong>{fiveHourUnlimited ? "无限" : `剩 ${formatPercent(snapshot?.primaryWindow?.remainingPercent)} · 用 ${formatPercent(snapshot?.primaryWindow?.usedPercent)}`}</strong>
        <small>{fiveHourUnlimited ? "无需重置" : `重置 ${formatCompactDateTime(snapshot?.primaryWindow?.resetAt)}`}</small>
      </div>
      <div class="top-hover-row">
        <span>7d</span>
        <strong>剩 {formatPercent(snapshot?.secondaryWindow?.remainingPercent)} · 用 {formatPercent(snapshot?.secondaryWindow?.usedPercent)}</strong>
        <small>重置 {formatCompactDateTime(snapshot?.secondaryWindow?.resetAt)}</small>
      </div>
      <p>重置次数 {resetCount}</p>
    </div>
  {/if}
</section>
