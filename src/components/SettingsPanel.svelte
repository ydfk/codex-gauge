<script lang="ts">
  import type { AppConfig } from "../lib/types";

  export let config: AppConfig | null = null;
  export let onsave: (config: AppConfig) => void;
  export let onback: () => void;

  $: draft = config ? structuredClone(config) : null;

  function save() {
    if (draft) onsave(draft);
  }
</script>

<section class="settings-panel">
  <header>
    <div>
      <span class="eyebrow">设置</span>
      <h1>Codex Gauge</h1>
    </div>
    <button type="button" onclick={onback}>返回</button>
  </header>

  {#if draft}
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
        <span>始终置顶</span>
        <input type="checkbox" bind:checked={draft.general.alwaysOnTop} onchange={save} />
      </label>

      <label class="toggle">
        <span>启动时显示桌面浮窗</span>
        <input type="checkbox" bind:checked={draft.general.showOnStartup} onchange={save} />
      </label>

      <label class="toggle">
        <span>显示顶部小浮条</span>
        <input type="checkbox" bind:checked={draft.general.topStatusEnabled} onchange={save} />
      </label>

      <label class="toggle">
        <span>防 OLED 烧屏微位移</span>
        <input type="checkbox" bind:checked={draft.general.oledShiftEnabled} onchange={save} />
      </label>

      <label class="toggle">
        <span>锁定位置</span>
        <input type="checkbox" bind:checked={draft.general.lockPosition} onchange={save} />
      </label>

      <label class="toggle">
        <span>开机启动</span>
        <input type="checkbox" bind:checked={draft.general.startOnBoot} onchange={save} />
      </label>
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
          <option value="api">API</option>
          <option value="app-server">app-server</option>
        </select>
      </label>
    </div>
  {:else}
    <p class="message">设置加载中</p>
  {/if}
</section>
