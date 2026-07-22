<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import DetailPanel from "../components/DetailPanel.svelte";
  import FloatingWidget from "../components/FloatingWidget.svelte";
  import MacMenuPanel from "../components/MacMenuPanel.svelte";
  import SettingsPanel from "../components/SettingsPanel.svelte";
  import TopStatusWidget from "../components/TopStatusWidget.svelte";
  import {
    getConfig,
    getAppVersion,
    getSnapshot,
    checkUpdate,
    installUpdate,
    openCodexLogin,
    refreshSnapshot,
    saveConfig,
    setTopContextMenu,
    setWindowMode,
    showWindow,
    hideWindow,
    toggleWindowVisible,
    quitApp,
    moveWindowForOled,
  } from "../lib/api";
  import type { AppConfig, CodexUsageSnapshot, UpdateCheckResult } from "../lib/types";

  let snapshot: CodexUsageSnapshot | null = null;
  let config: AppConfig | null = null;
  let message = "";
  let updateStatus: UpdateCheckResult | null = null;
  let lastUpdateCheckAt = 0;
  let appVersion = "";
  let refreshTimer: number | null = null;
  let oledTimer: number | null = null;
  let oledStep = 0;
  let oledOffset = { x: 0, y: 0 };
  let contextMenu: { x: number; y: number } | null = null;
  let topDetailsOpen = false;
  let macPanelView: "detail" | "settings" = "detail";
  const currentWindow = getCurrentWindow();
  const windowLabel = currentWindow.label;
  const isTopWindow = windowLabel === "top";
  const isMainWindow = windowLabel === "main";
  const isDetailWindow = windowLabel === "detail";
  const isSettingsWindow = windowLabel === "settings";
  const isMacMenuWindow = windowLabel === "menubar";
  const isOledWindow = isMainWindow || isTopWindow;
  $: currentAlwaysOnTop = isTopWindow
    ? (config?.general.topAlwaysOnTop ?? false)
    : (config?.general.mainAlwaysOnTop ?? false);
  $: currentPositionLocked = isTopWindow
    ? (config?.general.topLockPosition ?? false)
    : (config?.general.lockPosition ?? false);

  onMount(() => {
    void bootstrap();

    const unlisteners = [
      listen("codex-gauge-refresh", () => void refresh()),
      listen("codex-gauge-toggle-always-on-top", () => void toggleAlwaysOnTop()),
      listen("codex-gauge-toggle-lock", () => void toggleLockPosition()),
      listen("codex-gauge-toggle-start-on-boot", () => void toggleStartOnBoot()),
      listen("codex-gauge-open-login", () => void openCodexLogin()),
      listen("codex-gauge-check-update", () => void checkForUpdate(false)),
      listen("codex-gauge-install-update", () => void installAvailableUpdate()),
      listen("codex-gauge-config-updated", () => void reloadConfig()),
      listen<CodexUsageSnapshot>("codex-gauge-snapshot-updated", (event) => {
        snapshot = event.payload;
        message = "";
      }),
      listen("codex-gauge-open-macos-settings", () => {
        macPanelView = "settings";
      }),
      listen("codex-gauge-open-macos-detail", () => {
        macPanelView = "detail";
        if (config?.update.autoCheck && Date.now() - lastUpdateCheckAt >= 15 * 60 * 1000) {
          void checkForUpdate(true);
        }
      }),
    ];
    const handleKeydown = (event: KeyboardEvent) => {
      if (isMacMenuWindow && event.key === "Escape") void hideWindow("menubar");
    };
    window.addEventListener("keydown", handleKeydown);

    return () => {
      if (refreshTimer) window.clearInterval(refreshTimer);
      if (oledTimer) window.clearInterval(oledTimer);
      void resetOledShift();
      window.removeEventListener("keydown", handleKeydown);
      void Promise.all(unlisteners).then((items) => items.forEach((unlisten) => unlisten()));
    };
  });

  async function bootstrap() {
    try {
      [config, appVersion] = await Promise.all([getConfig(), getAppVersion()]);
      if (isMainWindow) await setWindowMode(false);
      await refresh();
      if ((isMainWindow || isMacMenuWindow) && config.update.autoCheck) void checkForUpdate(true);
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
    if (isTopWindow || isDetailWindow) return;
    lastUpdateCheckAt = Date.now();
    try {
      if (!silent) message = "检查更新中";
      updateStatus = await checkUpdate();
      if (!silent || updateStatus.available) message = updateStatus.message;
    } catch {
      if (!silent) message = "检查更新失败";
    }
  }

  async function installAvailableUpdate() {
    if (isTopWindow || isDetailWindow) return;
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
    refreshTimer = null;
    if (isMacMenuWindow) return;
    const seconds = config?.general.refreshIntervalSeconds ?? 60;
    refreshTimer = window.setInterval(() => void refresh(), Math.max(30, seconds) * 1000);
  }

  async function updateConfig(nextConfig: AppConfig) {
    const savedConfig = await saveConfig(nextConfig);
    config = savedConfig;
    scheduleRefresh();
    scheduleOledShift();
    return savedConfig;
  }

  async function reloadConfig() {
    config = await getConfig();
    scheduleRefresh();
    scheduleOledShift();
  }

  async function toggleLockPosition() {
    if (!config) return;
    const general = isTopWindow
      ? { ...config.general, topLockPosition: !config.general.topLockPosition }
      : { ...config.general, lockPosition: !config.general.lockPosition };
    await updateConfig({
      ...config,
      general,
    });
  }

  async function toggleAlwaysOnTop() {
    if (!config) return;
    const general = isTopWindow
      ? { ...config.general, topAlwaysOnTop: !config.general.topAlwaysOnTop }
      : { ...config.general, mainAlwaysOnTop: !config.general.mainAlwaysOnTop };
    await updateConfig({
      ...config,
      general,
    });
  }

  async function toggleStartOnBoot() {
    if (!config) return;
    await updateConfig({
      ...config,
      general: { ...config.general, startOnBoot: !config.general.startOnBoot },
    });
  }

  async function openDetailWindow() {
    await toggleWindowVisible("detail");
  }

  async function openSettingsWindow() {
    await showWindow("settings");
  }

  async function refreshFromSettings() {
    await refresh();
  }

  function scheduleOledShift() {
    if (oledTimer) window.clearInterval(oledTimer);
    oledTimer = null;
    if (
      !isOledWindow ||
      !config?.general.oledShiftEnabled ||
      (isMainWindow && config.general.lockPosition) ||
      (isTopWindow && config.general.topLockPosition)
    ) {
      void resetOledShift();
      return;
    }

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
    try {
      await moveWindowForOled(windowLabel as "main" | "top", dx, dy);
      oledStep += 1;
      oledOffset = { x: oledOffset.x + dx, y: oledOffset.y + dy };
    } catch {
      // 窗口位置微调失败不影响用量显示。
    }
  }

  async function resetOledShift() {
    if (!isOledWindow || (oledOffset.x === 0 && oledOffset.y === 0)) return;
    try {
      await moveWindowForOled(windowLabel as "main" | "top", -oledOffset.x, -oledOffset.y);
      oledOffset = { x: 0, y: 0 };
      oledStep = 0;
    } catch {
      // 退出或窗口销毁时无法复位不影响下次启动。
    }
  }

  function openContextMenu(event: MouseEvent) {
    event.preventDefault();
    if (isMacMenuWindow) return;
    const width = 160;
    const height = 58;
    const x = Math.max(4, Math.min(event.clientX, window.innerWidth - width - 4));
    const y = isTopWindow ? 29 : Math.max(4, Math.min(event.clientY, window.innerHeight - height - 4));
    contextMenu = { x, y };
    if (isTopWindow) {
      topDetailsOpen = false;
      void setTopContextMenu(true);
    }
  }

  async function toggleTopDetails(open: boolean) {
    if (!isTopWindow || contextMenu) return;
    topDetailsOpen = open;
    await setTopContextMenu(open);
  }

  async function refreshFromContext() {
    closeContextMenu();
    await refresh();
  }

  async function toggleAlwaysOnTopFromContext() {
    closeContextMenu();
    await toggleAlwaysOnTop();
  }

  async function toggleLockFromContext() {
    closeContextMenu();
    await toggleLockPosition();
  }

  async function closeCurrentWindow() {
    closeContextMenu();
    await hideWindow(windowLabel);
  }

  function closeContextMenu() {
    contextMenu = null;
    if (isTopWindow) {
      topDetailsOpen = false;
      void setTopContextMenu(false);
    }
  }
</script>

<main
  class:top-window={isTopWindow}
  class:top-details-open={topDetailsOpen}
  class:context-open={!!contextMenu}
  class:panel-window={isDetailWindow || isSettingsWindow}
  class:settings-window={isSettingsWindow}
  class:macos-menubar-window={isMacMenuWindow}
  style={`--panel-opacity: ${config?.general.opacity ?? 0.92}`}
  onpointerdown={closeContextMenu}
  oncontextmenu={openContextMenu}
>
  {#if isMacMenuWindow}
    <MacMenuPanel
      {snapshot}
      {config}
      {appVersion}
      {updateStatus}
      {message}
      view={macPanelView}
      onviewchange={(view) => (macPanelView = view)}
      onsave={updateConfig}
      onrefresh={refresh}
      onlogin={openCodexLogin}
      oncheckupdate={() => checkForUpdate(false)}
      oninstallupdate={installAvailableUpdate}
      onquit={() => void quitApp()}
    />
  {:else if isTopWindow}
    <TopStatusWidget
      {snapshot}
      locked={config?.general.topLockPosition ?? false}
      detailsOpen={topDetailsOpen}
      onmenu={openContextMenu}
      ondetail={() => void showWindow("detail")}
      onhoverchange={(open) => void toggleTopDetails(open)}
    />
  {:else if isDetailWindow}
    <DetailPanel
      {snapshot}
      onsettings={() => void openSettingsWindow()}
      onrefresh={() => refresh()}
      onlogin={() => openCodexLogin()}
      onclose={() => void hideWindow("detail")}
    />
  {:else if isSettingsWindow}
    <SettingsPanel
      {config}
      {appVersion}
      {updateStatus}
      onsave={updateConfig}
      oncheckupdate={() => checkForUpdate(false)}
      oninstallupdate={installAvailableUpdate}
      onrefresh={() => void refreshFromSettings()}
      ontogglemain={() => void toggleWindowVisible("main")}
      ontoggletop={() => void toggleWindowVisible("top")}
      onopendetail={() => void showWindow("detail")}
      onquit={() => void quitApp()}
      onback={() => void hideWindow("settings")}
    />
  {:else}
    <FloatingWidget
      {snapshot}
      {message}
      locked={config?.general.lockPosition ?? false}
      onopen={() => void openDetailWindow()}
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
      <button
        type="button"
        role="menuitemcheckbox"
        aria-checked={currentAlwaysOnTop}
        onclick={() => void toggleAlwaysOnTopFromContext()}
      >{currentAlwaysOnTop ? "取消置顶" : "置顶"}</button>
      <button
        type="button"
        role="menuitemcheckbox"
        aria-checked={currentPositionLocked}
        onclick={() => void toggleLockFromContext()}
      >{currentPositionLocked ? "取消固定" : "固定位置"}</button>
      <button type="button" role="menuitem" onclick={() => void refreshFromContext()}>刷新</button>
      <button type="button" role="menuitem" onclick={() => void closeCurrentWindow()}>关闭</button>
    </div>
  {/if}
</main>
