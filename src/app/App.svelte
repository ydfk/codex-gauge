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
    checkUpdate,
    installUpdate,
    openCodexLogin,
    refreshSnapshot,
    saveConfig,
    setTopContextMenu,
    setWindowMode,
  } from "../lib/api";
  import type { AppConfig, CodexUsageSnapshot, UpdateCheckResult } from "../lib/types";

  let snapshot: CodexUsageSnapshot | null = null;
  let config: AppConfig | null = null;
  let expanded = false;
  let settingsOpen = false;
  let message = "";
  let updateStatus: UpdateCheckResult | null = null;
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
      listen("codex-gauge-check-update", () => void checkForUpdate(false)),
      listen("codex-gauge-install-update", () => void installAvailableUpdate()),
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
      await refresh();
      if (!isTopWindow && config.update.autoCheck) void checkForUpdate(true);
      scheduleRefresh();
      scheduleOledShift();
    } catch {
      message = "初始化失败";
    }
  }

  async function refresh() {
    try {
      snapshot = await (snapshot ? refreshSnapshot() : getSnapshot());
      message = "";
    } catch {
      message = "刷新失败";
    }
  }

  async function checkForUpdate(silent: boolean) {
    if (isTopWindow) return;
    try {
      if (!silent) message = "检查更新中";
      updateStatus = await checkUpdate();
      if (!silent || updateStatus.available) message = updateStatus.message;
    } catch {
      if (!silent) message = "检查更新失败";
    }
  }

  async function installAvailableUpdate() {
    if (isTopWindow) return;
    try {
      message = "安装更新中";
      updateStatus = await installUpdate();
      message = updateStatus.message;
    } catch {
      message = "安装更新失败";
    }
  }

  function scheduleRefresh() {
    if (refreshTimer) window.clearInterval(refreshTimer);
    const seconds = config?.general.refreshIntervalSeconds ?? 60;
    refreshTimer = window.setInterval(() => void refresh(), Math.max(30, seconds) * 1000);
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
    const width = 90;
    const height = 62;
    const x = Math.max(4, Math.min(event.clientX, window.innerWidth - width - 4));
    const y = isTopWindow ? 29 : Math.max(4, Math.min(event.clientY, window.innerHeight - height - 4));
    contextMenu = { x, y };
    if (isTopWindow) void setTopContextMenu(true);
  }

  async function refreshFromContext() {
    closeContextMenu();
    await refresh();
  }

  async function closeCurrentWindow() {
    contextMenu = null;
    if (isTopWindow) await setTopContextMenu(false);
    await currentWindow.hide();
  }

  function closeContextMenu() {
    contextMenu = null;
    if (isTopWindow) void setTopContextMenu(false);
  }
</script>

<main
  class:expanded
  class:top-window={isTopWindow}
  class:context-open={!!contextMenu}
  class:settings-open={settingsOpen}
  style={`--panel-opacity: ${config?.general.opacity ?? 0.92}`}
  onpointerdown={closeContextMenu}
>
  {#if isTopWindow}
    <TopStatusWidget {snapshot} onmenu={openContextMenu} />
  {:else if expanded}
    {#if settingsOpen}
      <SettingsPanel
        {config}
        {updateStatus}
        onsave={(nextConfig) => updateConfig(nextConfig)}
        oncheckupdate={() => void checkForUpdate(false)}
        oninstallupdate={() => void installAvailableUpdate()}
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
      <button type="button" role="menuitem" onclick={() => void refreshFromContext()}>刷新</button>
      <button type="button" role="menuitem" onclick={() => void closeCurrentWindow()}>关闭</button>
    </div>
  {/if}
</main>
