<script lang="ts">
  import { checkUpdate, installUpdate } from "../lib/api";
  import type { AppConfig, UpdateCheckResult } from "../lib/types";

  export let config: AppConfig | null = null;
  export let onsave: (config: AppConfig) => void;
  export let onback: () => void;

  let updateMessage = "";
  let updateResult: UpdateCheckResult | null = null;
  let updating = false;

  $: draft = config ? structuredClone(config) : null;

  function save() {
    if (draft) onsave(draft);
  }

  async function handleCheckUpdate() {
    updating = true;
    try {
      updateResult = await checkUpdate();
      updateMessage = updateResult.version
        ? `${updateResult.message} 版本 ${updateResult.version}`
        : updateResult.message;
    } catch {
      updateMessage = "检查更新失败";
      updateResult = null;
    } finally {
      updating = false;
    }
  }

  async function handleInstallUpdate() {
    updating = true;
    try {
      const result = await installUpdate();
      updateMessage = result.message;
    } catch {
      updateMessage = "安装更新失败";
    } finally {
      updating = false;
    }
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
      <span>锁定位置</span>
      <input type="checkbox" bind:checked={draft.general.lockPosition} onchange={save} />
    </label>

    <label class="toggle">
      <span>开机启动</span>
      <input type="checkbox" bind:checked={draft.general.startOnBoot} onchange={save} />
    </label>

    <label class="toggle">
      <span>Token 统计</span>
      <input type="checkbox" bind:checked={draft.codex.enableUsageRead} onchange={save} />
    </label>

    <label class="toggle">
      <span>自动检查更新</span>
      <input type="checkbox" bind:checked={draft.update.autoCheck} onchange={save} />
    </label>

    <div class="update-box">
      <button type="button" onclick={handleCheckUpdate} disabled={updating}>检查更新</button>
      <span>{updateMessage || "版本 0.1.0"}</span>
    </div>

    {#if updateResult?.available}
      <div class="update-box">
        <button type="button" onclick={handleInstallUpdate} disabled={updating}>安装更新</button>
        <span>将从 GitHub Release 下载签名安装包</span>
      </div>
    {/if}
  {:else}
    <p class="message">设置加载中</p>
  {/if}
</section>
