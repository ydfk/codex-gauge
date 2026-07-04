<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { formatPercent, statusText } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let onmenu: (event: MouseEvent) => void;

  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";
  $: fiveHourTone = valueTone(snapshot?.primaryWindow?.remainingPercent);
  $: weeklyTone = valueTone(snapshot?.secondaryWindow?.remainingPercent);

  function valueTone(value: number | null | undefined) {
    if (value == null) return "tone-muted";
    if (value <= 5) return "tone-empty";
    if (value <= 15) return "tone-critical";
    if (value <= 30) return "tone-low";
    if (value <= 50) return "tone-mid";
    if (value <= 70) return "tone-good";
    return "tone-full";
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    void getCurrentWindow().startDragging();
  }
</script>

<section
  class="top-status-widget"
  role="presentation"
  title={statusText(snapshot)}
  oncontextmenu={onmenu}
  onpointerdown={handlePointerDown}
  data-tauri-drag-region
>
  <strong class={fiveHourTone} data-tauri-drag-region>5h {formatPercent(snapshot?.primaryWindow?.remainingPercent)}</strong>
  <span class={weeklyTone} data-tauri-drag-region>7d {formatPercent(snapshot?.secondaryWindow?.remainingPercent)}</span>
  <em data-tauri-drag-region>重置 {resetCount}</em>
</section>
