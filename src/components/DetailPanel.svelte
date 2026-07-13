<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    formatDateTime,
    formatLastUpdated,
    formatPercent,
    formatReset,
    sourceText,
    statusText,
  } from "../lib/format";
  import type { CodexUsageSnapshot } from "../lib/types";
  import UsageRing from "./UsageRing.svelte";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let onsettings: () => void;
  export let onrefresh: () => void;
  export let onlogin: () => void;
  export let onclose: () => void;

  let dragStart: { x: number; y: number; dragging: boolean } | null = null;
  let dragging = false;

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button, input")) return;
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

  function handleDoubleClick(event: MouseEvent) {
    if ((event.target as HTMLElement).closest("button, input, select, textarea")) return;
    onclose();
  }
</script>

<section
  class="detail-panel"
  class:dragging
  role="presentation"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
  ondblclick={handleDoubleClick}
>
  <div class="panel-aura" aria-hidden="true"></div>

  <header class="detail-hero">
    <div class="title-cluster">
      <span class="eyebrow">Codex telemetry</span>
      <h1>Codex Gauge</h1>
      <div class="hero-meta">
        <span>{statusText(snapshot)}</span>
        <span>{snapshot?.planType ?? "未知计划"}</span>
        <span>Credits {snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知"}</span>
      </div>
    </div>
    <div class="panel-actions">
      <button type="button" aria-label="刷新" title="刷新" onclick={onrefresh}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M20 11a8 8 0 1 0-2.34 5.66M20 11V5m0 6h-6" />
        </svg>
      </button>
      <button type="button" aria-label="设置" title="设置" onclick={onsettings}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 3v3m0 12v3m7.8-13.5-2.6 1.5M6.8 15l-2.6 1.5m15.6 0-2.6-1.5M6.8 9 4.2 7.5M12 15.5A3.5 3.5 0 1 0 12 8a3.5 3.5 0 0 0 0 7.5Z" />
        </svg>
      </button>
      <button type="button" aria-label="关闭详情" title="关闭" onclick={onclose}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 6l12 12M18 6 6 18" />
        </svg>
      </button>
    </div>
  </header>

  {#if snapshot?.status === "not_logged_in" || snapshot?.status === "invalid_auth"}
    <button class="login-button" type="button" onclick={onlogin}>
      <span>打开 Codex 登录</span>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 12h14m-5-5 5 5-5 5" />
      </svg>
    </button>
  {/if}

  <div class="cockpit-grid">
    <UsageRing
      label="5h"
      window={snapshot?.primaryWindow ?? null}
      unlimited={snapshot?.primaryWindowUnlimited ?? false}
    />
    <UsageRing label="7d" window={snapshot?.secondaryWindow ?? null} />
  </div>

  <div class="quota-board">
    <div>
      <span>5 小时使用</span>
      <strong>{snapshot?.primaryWindowUnlimited ? "无限" : formatPercent(snapshot?.primaryWindow?.usedPercent)}</strong>
      <small>{snapshot?.primaryWindowUnlimited ? "无用量限制" : `剩余 ${formatPercent(snapshot?.primaryWindow?.remainingPercent)} · ${formatReset(snapshot?.primaryWindow?.resetAt)}`}</small>
      <em>{snapshot?.primaryWindowUnlimited ? "无需重置" : `重置 ${formatDateTime(snapshot?.primaryWindow?.resetAt)}`}</em>
    </div>
    <div>
      <span>7d 使用</span>
      <strong>{formatPercent(snapshot?.secondaryWindow?.usedPercent)}</strong>
      <small>剩余 {formatPercent(snapshot?.secondaryWindow?.remainingPercent)} · {formatReset(snapshot?.secondaryWindow?.resetAt)}</small>
      <em>重置 {formatDateTime(snapshot?.secondaryWindow?.resetAt)}</em>
    </div>
  </div>

  <div class="metric-grid">
    <div>
      <span>Credits</span>
      <strong>{snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知"}</strong>
    </div>
    <div>
      <span>Credit Reset</span>
      <strong>{formatDateTime(snapshot?.credits?.resetAt)}</strong>
    </div>
    <div>
      <span>Plan</span>
      <strong>{snapshot?.planType ?? "未知"}</strong>
    </div>
    <div>
      <span>状态</span>
      <strong>{statusText(snapshot)}</strong>
    </div>
    <div>
      <span>5h 窗口</span>
      <strong>{snapshot?.primaryWindowUnlimited ? "无限" : snapshot?.primaryWindow?.windowDurationSeconds != null ? `${snapshot.primaryWindow.windowDurationSeconds}s` : "未知"}</strong>
    </div>
    <div>
      <span>7d 窗口</span>
      <strong>{snapshot?.secondaryWindow?.windowDurationSeconds ?? "未知"}s</strong>
    </div>
    <div>
      <span>触发类型</span>
      <strong>{snapshot?.rateLimitReachedType ?? "无"}</strong>
    </div>
    <div>
      <span>来源</span>
      <strong>{sourceText(snapshot)}</strong>
    </div>
  </div>

  <div class="metric-grid credit-list">
    {#if snapshot?.credits?.items?.length}
      {#each snapshot.credits.items as credit}
        <div>
          <span>{credit.title ?? "未知标题"}</span>
          <strong>{credit.status ?? "未知状态"}</strong>
          <small>Granted {credit.grantedAt ?? "未知"}</small>
          <em>Expires {credit.expiresAt ?? "未知"}</em>
        </div>
      {/each}
    {:else}
      <div>
        <span>Credit 明细</span>
        <strong>未知</strong>
      </div>
    {/if}
  </div>

  <footer class="detail-footer">
    <span>最后刷新：{formatLastUpdated(snapshot?.updatedAt)}</span>
    <span>数据来源：{sourceText(snapshot)}</span>
  </footer>
</section>
