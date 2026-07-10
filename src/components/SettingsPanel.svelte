<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { AppConfig, UpdateCheckResult } from "../lib/types";

  export let config: AppConfig | null = null;
  export let updateStatus: UpdateCheckResult | null = null;
  export let onsave: (config: AppConfig) => Promise<AppConfig>;
  export let oncheckupdate: () => Promise<void>;
  export let oninstallupdate: () => Promise<void>;
  export let onrefresh: () => void;
  export let ontogglemain: () => void;
  export let ontoggletop: () => void;
  export let onopendetail: () => void;
  export let onquit: () => void;
  export let onback: () => void;

  $: draft = config ? structuredClone(config) : null;
  let dragStart: { x: number; y: number; dragging: boolean } | null = null;
  let dragging = false;
  let updateAction = "";
  let saveError = "";

  async function save() {
    if (!draft) return;
    try {
      await onsave(structuredClone(draft));
      saveError = "";
    } catch {
      draft = config ? structuredClone(config) : null;
      saveError = "保存设置失败";
    }
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button, input, select, textarea")) return;
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

  async function runUpdateAction(label: string, action: () => Promise<void>) {
    updateAction = label;
    try {
      await action();
    } finally {
      updateAction = "";
    }
  }
</script>

<section
  class="settings-panel"
  class:dragging
  role="presentation"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
>
  <header>
    <div>
      <span class="eyebrow">设置</span>
      <h1>Codex Gauge</h1>
    </div>
    <div class="panel-actions">
      <button type="button" aria-label="关闭设置" title="关闭" onclick={onback}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 6l12 12M18 6 6 18" />
        </svg>
      </button>
    </div>
  </header>

  {#if draft}
    <div class="settings-content">
      <div class="settings-section">
        <span class="section-title">浮窗</span>
        <label>
          <span>刷新间隔</span>
          <input
            type="number"
            min="30"
            max="600"
            bind:value={draft.general.refreshIntervalSeconds}
            oninput={save}
          />
        </label>

        <label>
          <span>透明度</span>
          <input
            type="range"
            min="0.7"
            max="1"
            step="0.01"
            bind:value={draft.general.opacity}
            oninput={save}
          />
        </label>

        <label class="toggle">
          <span>桌面浮窗置顶</span>
          <input type="checkbox" bind:checked={draft.general.mainAlwaysOnTop} onchange={save} />
        </label>

        <label class="toggle">
          <span>顶部浮窗置顶</span>
          <input type="checkbox" bind:checked={draft.general.topAlwaysOnTop} onchange={save} />
        </label>

        <div class="setting-pair">
          <label class="toggle">
            <span>显示桌面浮窗</span>
            <input type="checkbox" bind:checked={draft.general.showOnStartup} onchange={save} />
          </label>

          <label class="toggle">
            <span>显示顶部小浮条</span>
            <input type="checkbox" bind:checked={draft.general.topStatusEnabled} onchange={save} />
          </label>
        </div>

        <label class="toggle">
          <span>防 OLED 烧屏微位移</span>
          <input type="checkbox" bind:checked={draft.general.oledShiftEnabled} onchange={save} />
        </label>

        <div class="setting-pair">
          <label class="toggle">
            <span>锁定桌面浮窗位置</span>
            <input type="checkbox" bind:checked={draft.general.lockPosition} onchange={save} />
          </label>

          <label class="toggle">
            <span>锁定顶部浮条位置</span>
            <input type="checkbox" bind:checked={draft.general.topLockPosition} onchange={save} />
          </label>
        </div>

        <label class="toggle">
          <span>开机启动</span>
          <input type="checkbox" bind:checked={draft.general.startOnBoot} onchange={save} />
        </label>
      </div>

      <div class="settings-stack">
        <div class="settings-section tray-actions">
          <span class="section-title">快捷操作</span>
          <div class="setting-action-row">
            <button type="button" onclick={ontogglemain}>显示/隐藏桌面浮窗</button>
            <button type="button" onclick={ontoggletop}>显示/隐藏顶部浮窗</button>
          </div>
          <div class="setting-action-row">
            <button type="button" onclick={onopendetail}>打开详细信息</button>
            <button type="button" onclick={onrefresh}>刷新用量</button>
          </div>
          <button class="quit-button" type="button" onclick={onquit}>退出 Codex Gauge</button>
        </div>

        <div class="settings-section">
          <span class="section-title">Codex</span>
          <label>
            <span>命令</span>
            <input type="text" bind:value={draft.codex.command} oninput={save} />
          </label>

          <label>
            <span>优先查询方式</span>
            <select bind:value={draft.codex.preferredProvider} onchange={save}>
              <option value="app-server">app-server</option>
              <option value="api">API</option>
            </select>
          </label>
        </div>

        <div class="settings-section">
          <span class="section-title">更新</span>
          <label class="toggle">
            <span>启动时自动检查</span>
            <input type="checkbox" bind:checked={draft.update.autoCheck} onchange={save} />
          </label>

          <label class="toggle">
            <span>发现更新后自动安装</span>
            <input type="checkbox" bind:checked={draft.update.autoInstall} onchange={save} />
          </label>

          <label class="setting-wide">
            <span>更新地址</span>
            <input type="text" bind:value={draft.update.endpoint} onchange={save} />
          </label>

          <div class="update-box">
            <div>
              <strong>{updateStatus?.available ? "发现新版本" : "更新状态"}</strong>
              <span>{updateAction || updateStatus?.message || "尚未检查"}</span>
            </div>
            <div class="settings-actions">
              <button
                type="button"
                disabled={!!updateAction}
                onclick={() => void runUpdateAction("正在检查更新", oncheckupdate)}
              >检查</button>
              <button
                type="button"
                disabled={!!updateAction}
                onclick={() => void runUpdateAction("正在检查并安装更新", oninstallupdate)}
              >
                更新
              </button>
            </div>
          </div>
        </div>
      </div>
      {#if saveError}
        <p class="settings-error" role="alert">{saveError}</p>
      {/if}
    </div>
  {:else}
    <p class="message">设置加载中</p>
  {/if}
</section>
