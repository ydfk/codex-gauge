<script lang="ts">
  import {
    formatDateTime,
    formatFullNumber,
    formatLastUpdated,
    formatPercent,
    formatReset,
    statusText,
  } from "../lib/format";
  import { resetCreditText } from "../lib/reset";
  import type { CodexGaugeSnapshot } from "../lib/types";
  import UsageRing from "./UsageRing.svelte";

  export let snapshot: CodexGaugeSnapshot | null = null;
  export let onsettings: () => void;
  export let onrefresh: () => void;
  export let onlogin: () => void;
  export let onclose: () => void;
</script>

<section class="detail-panel">
  <div class="panel-aura" aria-hidden="true"></div>

  <header class="detail-hero">
    <div class="title-cluster">
      <span class="eyebrow">Codex telemetry</span>
      <h1>Codex Gauge</h1>
      <div class="hero-meta">
        <span>{statusText(snapshot)}</span>
        <span>{snapshot?.planType ?? "未知计划"}</span>
        <span>{resetCreditText(snapshot?.reset)}</span>
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

  {#if snapshot?.status === "not_logged_in" || snapshot?.status === "codex_not_found"}
    <button class="login-button" type="button" onclick={onlogin}>
      <span>打开 Codex 登录</span>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 12h14m-5-5 5 5-5 5" />
      </svg>
    </button>
  {/if}

  <div class="cockpit-grid">
    <UsageRing label="5h" window={snapshot?.fiveHour ?? null} />
    <UsageRing label="1w" window={snapshot?.weekly ?? null} />
  </div>

  <div class="quota-board">
    <div>
      <span>5 小时</span>
      <strong>{formatPercent(snapshot?.fiveHour?.usedPercent)}</strong>
      <small>剩余 {formatPercent(snapshot?.fiveHour?.remainingPercent)} · {formatReset(snapshot?.fiveHour?.resetRemainingText)}</small>
      <em>重置 {formatDateTime(snapshot?.fiveHour?.resetsAt)}</em>
    </div>
    <div>
      <span>一周</span>
      <strong>{formatPercent(snapshot?.weekly?.usedPercent)}</strong>
      <small>剩余 {formatPercent(snapshot?.weekly?.remainingPercent)} · {formatReset(snapshot?.weekly?.resetRemainingText)}</small>
      <em>重置 {formatDateTime(snapshot?.weekly?.resetsAt)}</em>
    </div>
  </div>

  <div class="metric-grid">
    <div>
      <span>今日 5h 自动重置</span>
      <strong>{snapshot?.reset.todayAutoReset5h ?? 0} 次</strong>
    </div>
    <div>
      <span>本周 1w 自动重置</span>
      <strong>{snapshot?.reset.weekAutoResetWeekly ?? 0} 次</strong>
    </div>
    <div>
      <span>可用重置</span>
      <strong>{snapshot?.reset.availableResetCredits ?? "未知"} 次</strong>
    </div>
    <div>
      <span>疑似手动重置消耗</span>
      <strong>{snapshot?.reset.todayManualResetConsumed ?? 0} 次</strong>
    </div>
    <div>
      <span>今日 Token</span>
      <strong>{formatFullNumber(snapshot?.tokenUsage.todayTokens)}</strong>
    </div>
    <div>
      <span>本周 Token</span>
      <strong>{formatFullNumber(snapshot?.tokenUsage.weekTokens)}</strong>
    </div>
    <div>
      <span>全部 Token</span>
      <strong>{formatFullNumber(snapshot?.tokenUsage.lifetimeTokens)}</strong>
    </div>
    <div>
      <span>峰值日 Token</span>
      <strong>{formatFullNumber(snapshot?.tokenUsage.peakDailyTokens)}</strong>
    </div>
  </div>

  <footer class="detail-footer">
    <span>最后刷新：{formatLastUpdated(snapshot?.lastUpdatedAt)}</span>
    <span>数据来源：Codex App Server</span>
  </footer>
</section>
