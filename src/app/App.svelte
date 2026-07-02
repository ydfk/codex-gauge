<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import DetailPanel from "../components/DetailPanel.svelte";
  import FloatingWidget from "../components/FloatingWidget.svelte";
  import SettingsPanel from "../components/SettingsPanel.svelte";
  import {
    getConfig,
    getSnapshot,
    openCodexLogin,
    refreshSnapshot,
    saveConfig,
    setWindowMode,
  } from "../lib/api";
  import type { AppConfig, CodexGaugeSnapshot } from "../lib/types";
  import { startWindowDrag } from "../lib/window";

  let snapshot: CodexGaugeSnapshot | null = null;
  let config: AppConfig | null = null;
  let expanded = false;
  let settingsOpen = false;
  let busy = false;
  let message = "";
  let refreshTimer: number | null = null;

  onMount(() => {
    void bootstrap();

    const unlisteners = [
      listen("codex-gauge-refresh", () => void refresh()),
      listen("codex-gauge-toggle-lock", () => void toggleLockPosition()),
      listen("codex-gauge-toggle-start-on-boot", () => void toggleStartOnBoot()),
      listen("codex-gauge-toggle-auto-update", () => void toggleAutoUpdate()),
      listen("codex-gauge-open-login", () => void openCodexLogin()),
    ];

    return () => {
      if (refreshTimer) window.clearInterval(refreshTimer);
      void Promise.all(unlisteners).then((items) => items.forEach((unlisten) => unlisten()));
    };
  });

  async function bootstrap() {
    try {
      config = await getConfig();
      await setWindowMode(false);
      await refresh(false);
      scheduleRefresh();
    } catch {
      message = "初始化失败";
    }
  }

  async function refresh(showBusy = true) {
    if (showBusy) busy = true;
    try {
      snapshot = await (snapshot ? refreshSnapshot() : getSnapshot());
      message = "";
    } catch {
      message = "刷新失败";
    } finally {
      busy = false;
    }
  }

  function scheduleRefresh() {
    if (refreshTimer) window.clearInterval(refreshTimer);
    const seconds = config?.general.refreshIntervalSeconds ?? 60;
    refreshTimer = window.setInterval(() => void refresh(false), Math.max(30, seconds) * 1000);
  }

  async function updateConfig(nextConfig: AppConfig) {
    config = await saveConfig(nextConfig);
    scheduleRefresh();
  }

  async function toggleLockPosition() {
    if (!config) return;
    await updateConfig({
      ...config,
      general: { ...config.general, lockPosition: !config.general.lockPosition },
    });
  }

  async function toggleStartOnBoot() {
    if (!config) return;
    await updateConfig({
      ...config,
      general: { ...config.general, startOnBoot: !config.general.startOnBoot },
    });
  }

  async function toggleAutoUpdate() {
    if (!config) return;
    await updateConfig({
      ...config,
      update: { ...config.update, autoCheck: !config.update.autoCheck },
    });
  }

  async function toggleExpanded() {
    expanded = !expanded;
    settingsOpen = false;
    await setWindowMode(expanded);
  }

  async function collapsePanel() {
    expanded = false;
    settingsOpen = false;
    await setWindowMode(false);
  }
</script>

<main
  class:expanded
  class:settings-open={settingsOpen}
  onpointerdown={startWindowDrag}
  style={`--panel-opacity: ${config?.general.opacity ?? 0.92}`}
>
  {#if expanded}
    {#if settingsOpen}
      <SettingsPanel
        {config}
        onsave={(nextConfig) => updateConfig(nextConfig)}
        onback={() => (settingsOpen = false)}
      />
    {:else}
      <DetailPanel
        {snapshot}
        onsettings={() => (settingsOpen = true)}
        onrefresh={() => refresh()}
        onlogin={() => openCodexLogin()}
        onclose={() => void collapsePanel()}
      />
    {/if}
  {:else}
    <FloatingWidget
      {snapshot}
      {busy}
      {message}
      onopen={() => void toggleExpanded()}
      onrefresh={() => refresh()}
    />
  {/if}
</main>
