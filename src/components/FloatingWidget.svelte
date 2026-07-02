<script lang="ts">
  import { formatPercent, formatReset, formatTokens, statusText, usageLevel } from "../lib/format";
  import { resetCreditText } from "../lib/reset";
  import type { CodexGaugeSnapshot } from "../lib/types";
  import { startWindowDrag } from "../lib/window";

  export let snapshot: CodexGaugeSnapshot | null = null;
  export let busy = false;
  export let message = "";
  export let onopen: () => void;
  export let onrefresh: () => void;

  $: level = usageLevel(snapshot);
  $: fiveHourWidth = barWidth(snapshot?.fiveHour?.usedPercent);
  $: weeklyWidth = barWidth(snapshot?.weekly?.usedPercent);

  function barWidth(value: number | null | undefined) {
    return `${Math.max(0, Math.min(100, value ?? 0))}%`;
  }
</script>

<section class={`floating-widget level-${level}`} data-tauri-drag-region>
  <div class="glass-refraction" data-tauri-drag-region></div>
  <div
    class="widget-body"
    role="presentation"
    title="双击查看详情"
    data-tauri-drag-region
    onpointerdown={startWindowDrag}
    ondblclick={onopen}
  >
    <header class="widget-top" data-tauri-drag-region>
      <span class="brand-stack" data-tauri-drag-region>
        <span class="brand" data-tauri-drag-region>Codex Gauge</span>
        <span class="status-copy" data-tauri-drag-region>{statusText(snapshot)}</span>
      </span>
      <span class={`status-dot ${busy ? "pulse" : ""}`} aria-label={statusText(snapshot)} data-tauri-drag-region></span>
    </header>

    <div class="usage-matrix" data-tauri-drag-region>
      <div class="usage-line" data-tauri-drag-region>
        <div class="usage-card-head" data-tauri-drag-region>
          <span data-tauri-drag-region>5h</span>
          <strong data-tauri-drag-region>{formatPercent(snapshot?.fiveHour?.usedPercent)}</strong>
        </div>
        <div class="liquid-meter" data-tauri-drag-region>
          <span style={`width: ${fiveHourWidth}`} data-tauri-drag-region></span>
        </div>
        <small data-tauri-drag-region>{formatReset(snapshot?.fiveHour?.resetRemainingText)}</small>
      </div>
      <div class="usage-line" data-tauri-drag-region>
        <div class="usage-card-head" data-tauri-drag-region>
          <span data-tauri-drag-region>1w</span>
          <strong data-tauri-drag-region>{formatPercent(snapshot?.weekly?.usedPercent)}</strong>
        </div>
        <div class="liquid-meter" data-tauri-drag-region>
          <span style={`width: ${weeklyWidth}`} data-tauri-drag-region></span>
        </div>
        <small data-tauri-drag-region>{formatReset(snapshot?.weekly?.resetRemainingText)}</small>
      </div>
    </div>

    <footer class="widget-bottom" data-tauri-drag-region>
      <span class="credit-chip" data-tauri-drag-region>{resetCreditText(snapshot?.reset)}</span>
      <span data-tauri-drag-region>Today {formatTokens(snapshot?.tokenUsage.todayTokens)}</span>
    </footer>
  </div>

  <div class="quick-actions">
    <button type="button" title="刷新" aria-label="刷新" onclick={(event) => {
      event.stopPropagation();
      onrefresh();
    }}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M20 11a8 8 0 1 0-2.34 5.66M20 11V5m0 6h-6" />
      </svg>
    </button>
  </div>

  {#if message}
    <p class="message">{message}</p>
  {/if}
</section>
