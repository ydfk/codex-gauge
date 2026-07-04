<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { PhysicalPosition } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import DetailPanel from "../components/DetailPanel.svelte";
  import FloatingWidget from "../components/FloatingWidget.svelte";
  import SettingsPanel from "../components/SettingsPanel.svelte";
  import TopStatusWidget from "../components/TopStatusWidget.svelte";
  import {
    getConfig,
    getSnapshot,
    openCodexLogin,
    refreshSnapshot,
    saveConfig,
    setWindowMode,
  } from "../lib/api";
  import type { AppConfig, CodexUsageSnapshot } from "../lib/types";

  let snapshot: CodexUsageSnapshot | null = null;
  let config: AppConfig | null = null;
  let expanded = false;
  let settingsOpen = false;
  let busy = false;
  let message = "";
  let refreshTimer: number | null = null;
  let oledTimer: number | null = null;
  let oledStep = 0;
  let contextMenu: { x: number; y: number } | null = null;
  const currentWindow = getCurrentWindow();
  const windowLabel = currentWindow.label;
  const isTopWindow = windowLabel === "top";

  onMount(() => {
    void bootstrap();

    const unlisteners = [
      listen("codex-gauge-refresh", () => void refresh()),
      listen("codex-gauge-open-settings", () => void openSettingsPanel()),
      listen("codex-gauge-toggle-always-on-top", () => void toggleAlwaysOnTop()),
      listen("codex-gauge-toggle-lock", () => void toggleLockPosition()),
      listen("codex-gauge-toggle-start-on-boot", () => void toggleStartOnBoot()),
      listen("codex-gauge-toggle-auto-update", () => void toggleAutoUpdate()),
      listen("codex-gauge-open-login", () => void openCodexLogin()),
    ];

    return () => {
      if (refreshTimer) window.clearInterval(refreshTimer);
      if (oledTimer) window.clearInterval(oledTimer);
      void Promise.all(unlisteners).then((items) => items.forEach((unlisten) => unlisten()));
    };
  });

  async function bootstrap() {
    try {
      config = await getConfig();
      if (!isTopWindow) await setWindowMode(false);
      await refresh(false);
      scheduleRefresh();
      scheduleOledShift();
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
    scheduleOledShift();
  }

  async function toggleLockPosition() {
    if (!config) return;
    await updateConfig({
      ...config,
      general: { ...config.general, lockPosition: !config.general.lockPosition },
    });
  }

  async function toggleAlwaysOnTop() {
    if (!config) return;
    await updateConfig({
      ...config,
      general: { ...config.general, alwaysOnTop: !config.general.alwaysOnTop },
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

  async function openSettingsPanel() {
    expanded = true;
    settingsOpen = true;
    await setWindowMode(true);
  }

  async function collapsePanel() {
    expanded = false;
    settingsOpen = false;
    await setWindowMode(false);
  }

  function scheduleOledShift() {
    if (oledTimer) window.clearInterval(oledTimer);
    oledTimer = null;
    if (!config?.general.oledShiftEnabled) return;

    oledTimer = window.setInterval(() => void nudgeWindow(), 180_000);
  }

  async function nudgeWindow() {
    const offsets = [
      [1, 0],
      [1, 1],
      [0, 1],
      [-1, 1],
      [-1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
    ];
    const [dx, dy] = offsets[oledStep % offsets.length];
    oledStep += 1;
    try {
      const position = await currentWindow.outerPosition();
      await currentWindow.setPosition(new PhysicalPosition(position.x + dx, position.y + dy));
    } catch {
      // 窗口位置微调失败不影响用量显示。
    }
  }

  function openContextMenu(event: MouseEvent) {
    event.preventDefault();
    contextMenu = { x: event.clientX, y: event.clientY };
  }

  async function closeCurrentWindow() {
    contextMenu = null;
    await currentWindow.hide();
  }

  function closeContextMenu() {
    contextMenu = null;
  }
</script>

<main
  class:expanded
  class:top-window={isTopWindow}
  class:settings-open={settingsOpen}
  style={`--panel-opacity: ${config?.general.opacity ?? 0.92}`}
  onpointerdown={closeContextMenu}
>
  {#if isTopWindow}
    <TopStatusWidget {snapshot} {busy} onmenu={openContextMenu} />
  {:else if expanded}
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
      onmenu={openContextMenu}
    />
  {/if}

  {#if contextMenu}
    <div
      class="float-context-menu"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px`}
      role="menu"
      tabindex="-1"
      onpointerdown={(event) => event.stopPropagation()}
    >
      <button type="button" role="menuitem" onclick={() => void refresh()}>刷新</button>
      <button type="button" role="menuitem" onclick={() => void closeCurrentWindow()}>关闭</button>
    </div>
  {/if}
</main>
