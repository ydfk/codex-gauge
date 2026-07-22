<script lang="ts">
  import {
    formatCompactDateTime,
    formatLastUpdated,
    formatPercent,
    remainingTone,
    sourceText,
    statusText,
  } from "../lib/format";
  import type { AppConfig, CodexUsageSnapshot, UpdateCheckResult } from "../lib/types";

  export let snapshot: CodexUsageSnapshot | null = null;
  export let config: AppConfig | null = null;
  export let appVersion = "";
  export let updateStatus: UpdateCheckResult | null = null;
  export let message = "";
  export let view: "detail" | "settings" = "detail";
  export let onviewchange: (view: "detail" | "settings") => void;
  export let onsave: (config: AppConfig) => Promise<AppConfig>;
  export let onrefresh: () => Promise<void>;
  export let onlogin: () => Promise<void>;
  export let oncheckupdate: () => Promise<void>;
  export let oninstallupdate: () => Promise<void>;
  export let onquit: () => void;

  $: draft = config ? structuredClone(config) : null;
  $: fiveHourUnlimited = snapshot?.primaryWindowUnlimited ?? false;
  $: resetCount = snapshot?.credits?.availableCount ?? snapshot?.credits?.resetCredits ?? "未知";
  $: currentVersion = appVersion ? `v${appVersion}` : "未知";
  $: availableVersion = updateStatus?.available && updateStatus.version
    ? `v${updateStatus.version}`
    : null;

  let refreshing = false;
  let updateAction = "";
  let saveError = "";

  function meterWidth(value: number | null | undefined) {
    return `${Math.max(0, Math.min(100, value ?? 0))}%`;
  }

  function creditTitle(title: string | null | undefined) {
    if (!title) return "重置券";
    if (title.toLowerCase().includes("full reset")) return "完整重置（5h + 7d）";
    return title;
  }

  function creditStatus(status: string | null | undefined) {
    if (status === "available") return "可用";
    if (status === "consumed" || status === "used") return "已使用";
    if (status === "expired") return "已过期";
    return status ?? "未知";
  }

  async function refresh() {
    refreshing = true;
    try {
      await onrefresh();
    } finally {
      refreshing = false;
    }
  }

  async function save() {
    if (!draft) return;
    try {
      await onsave(structuredClone(draft));
      saveError = "";
    } catch {
      draft = config ? structuredClone(config) : null;
      saveError = "设置保存失败";
    }
  }

  async function runUpdate(label: string, action: () => Promise<void>) {
    updateAction = label;
    try {
      await action();
    } finally {
      updateAction = "";
    }
  }
</script>

<section class="mac-menu-panel" aria-label="Codex Gauge">
  <header class="mac-panel-header">
    <div>
      <strong>Codex Gauge</strong>
      <span>{message || statusText(snapshot)}</span>
    </div>
    {#if view === "detail"}
      <button
        class:spinning={refreshing}
        type="button"
        aria-label="刷新用量"
        title="刷新用量"
        onclick={() => void refresh()}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M20 11a8 8 0 1 0-2.34 5.66M20 11V5m0 6h-6" />
        </svg>
      </button>
    {:else}
      <button type="button" aria-label="返回详细信息" title="返回" onclick={() => onviewchange("detail")}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M15 18 9 12l6-6" />
        </svg>
      </button>
    {/if}
  </header>

  {#if view === "detail"}
    <div class="mac-panel-scroll">
      {#if availableVersion}
        <button
          class="mac-update-notice"
          type="button"
          disabled={!!updateAction}
          onclick={() => void runUpdate("正在下载并安装", oninstallupdate)}
        >
          <span>
            <small>发现新版本</small>
            <strong>{currentVersion} → {availableVersion}</strong>
          </span>
          <em>{updateAction || "点击安装并重启"}</em>
        </button>
      {/if}

      {#if snapshot?.status === "not_logged_in" || snapshot?.status === "invalid_auth"}
        <button class="mac-login-button" type="button" onclick={() => void onlogin()}>
          <span>未检测到有效登录状态</span>
          <strong>打开 Codex 登录</strong>
        </button>
      {/if}

      <div class="mac-quota-stack">
        <article>
          <div class="mac-quota-heading">
            <div><span>5 小时</span><small>短周期额度</small></div>
            <strong class={fiveHourUnlimited ? "tone-full" : remainingTone(snapshot?.primaryWindow?.remainingPercent)}>
              {fiveHourUnlimited ? "无限" : formatPercent(snapshot?.primaryWindow?.remainingPercent)}
            </strong>
          </div>
          <div class={`mac-quota-meter ${fiveHourUnlimited ? "tone-full" : remainingTone(snapshot?.primaryWindow?.remainingPercent)}`}>
            <i style={`width: ${fiveHourUnlimited ? "100%" : meterWidth(snapshot?.primaryWindow?.remainingPercent)}`}></i>
          </div>
          <div class="mac-quota-meta">
            <span>{fiveHourUnlimited ? "无需重置" : `已用 ${formatPercent(snapshot?.primaryWindow?.usedPercent)}`}</span>
            <span>{fiveHourUnlimited ? "当前套餐不受限制" : `重置 ${formatCompactDateTime(snapshot?.primaryWindow?.resetAt)}`}</span>
          </div>
        </article>

        <article>
          <div class="mac-quota-heading">
            <div><span>7 天</span><small>长周期额度</small></div>
            <strong class={remainingTone(snapshot?.secondaryWindow?.remainingPercent)}>
              {formatPercent(snapshot?.secondaryWindow?.remainingPercent)}
            </strong>
          </div>
          <div class={`mac-quota-meter ${remainingTone(snapshot?.secondaryWindow?.remainingPercent)}`}>
            <i style={`width: ${meterWidth(snapshot?.secondaryWindow?.remainingPercent)}`}></i>
          </div>
          <div class="mac-quota-meta">
            <span>已用 {formatPercent(snapshot?.secondaryWindow?.usedPercent)}</span>
            <span>重置 {formatCompactDateTime(snapshot?.secondaryWindow?.resetAt)}</span>
          </div>
        </article>
      </div>

      <div class="mac-facts">
        <div><span>可用重置</span><strong>{resetCount}</strong></div>
        <div><span>当前计划</span><strong>{snapshot?.planType ?? "未知"}</strong></div>
        <div><span>数据来源</span><strong>{sourceText(snapshot)}</strong></div>
      </div>

      {#if snapshot?.credits?.items?.length}
        <div class="mac-credit-list">
          <span class="mac-section-title">重置券明细</span>
          {#each snapshot.credits.items as credit}
            <div>
              <strong>{creditTitle(credit.title)}</strong>
              <span>{creditStatus(credit.status)}</span>
              <small>{credit.expiresAt ? `到期 ${credit.expiresAt}` : "未提供到期时间"}</small>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <footer class="mac-panel-footer">
      <span>更新于 {formatLastUpdated(snapshot?.updatedAt)}</span>
      <div>
        <button type="button" onclick={() => onviewchange("settings")}>设置</button>
        <button type="button" onclick={onquit}>退出</button>
      </div>
    </footer>
  {:else if draft}
    <div class="mac-settings-scroll">
      <div class="mac-setting-section">
        <span class="mac-section-title">菜单栏</span>
        <label>
          <span>显示内容</span>
          <select bind:value={draft.macos.menuBarDisplay} onchange={save}>
            <option value="fiveAndSeven">图标 + 5h + 7d</option>
            <option value="fiveHour">图标 + 5h</option>
            <option value="iconOnly">仅图标</option>
          </select>
        </label>
        <label>
          <span>刷新间隔</span>
          <select bind:value={draft.general.refreshIntervalSeconds} onchange={save}>
            <option value={30}>30 秒</option>
            <option value={60}>1 分钟</option>
            <option value={120}>2 分钟</option>
            <option value={300}>5 分钟</option>
            <option value={600}>10 分钟</option>
          </select>
        </label>
        <label class="mac-toggle-row">
          <span>登录时启动</span>
          <input type="checkbox" bind:checked={draft.general.startOnBoot} onchange={save} />
        </label>
      </div>

      <div class="mac-setting-section">
        <span class="mac-section-title">Codex</span>
        <label class="mac-setting-wide">
          <span>命令路径</span>
          <input type="text" bind:value={draft.codex.command} onchange={save} />
        </label>
        <label>
          <span>优先查询方式</span>
          <select bind:value={draft.codex.preferredProvider} onchange={save}>
            <option value="app-server">App Server</option>
            <option value="api">AuthJson API</option>
          </select>
        </label>
      </div>

      <div class="mac-setting-section">
        <span class="mac-section-title">更新</span>
        <label class="mac-toggle-row">
          <span>启动时自动检查</span>
          <input type="checkbox" bind:checked={draft.update.autoCheck} onchange={save} />
        </label>
        <div class="mac-update-box">
          <div>
            <strong>{availableVersion ? `${currentVersion} → ${availableVersion}` : `当前 ${currentVersion}`}</strong>
            <span>{updateAction || updateStatus?.message || "尚未检查更新"}</span>
          </div>
          <div>
            <button
              type="button"
              disabled={!!updateAction}
              onclick={() => void runUpdate("正在检查", oncheckupdate)}
            >检查</button>
            <button
              type="button"
              disabled={!availableVersion || !!updateAction}
              onclick={() => void runUpdate("正在安装", oninstallupdate)}
            >{availableVersion ? "安装" : "已是最新"}</button>
          </div>
        </div>
      </div>

      {#if saveError}<p class="mac-settings-error" role="alert">{saveError}</p>{/if}
    </div>
  {/if}
</section>
