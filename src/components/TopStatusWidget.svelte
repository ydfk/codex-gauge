<script lang="ts">
  import { PhysicalPosition } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { formatPercent, statusText } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let onmenu: (event: MouseEvent) => void;

  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";
  $: fiveHourTone = valueTone(snapshot?.primaryWindow?.remainingPercent);
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
    if (event.button !== 0) return;
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
    if (!horizontalDrag) return;
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
</script>

<section
  class="top-status-widget"
  class:dragging
  role="presentation"
  title={statusText(snapshot)}
  oncontextmenu={handleContextMenu}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerEnd}
  onpointercancel={handlePointerEnd}
>
  <strong class={fiveHourTone}>5h {formatPercent(snapshot?.primaryWindow?.remainingPercent)}</strong>
  <span class={weeklyTone}>7d {formatPercent(snapshot?.secondaryWindow?.remainingPercent)}</span>
  <em>重置 {resetCount}</em>
</section>
