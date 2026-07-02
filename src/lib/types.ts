export type UsageWindow = {
  label: "5h" | "1w" | "other";
  usedPercent: number | null;
  remainingPercent: number | null;
  windowDurationMins: number | null;
  resetsAt: number | null;
  resetRemainingText: string | null;
};

export type TokenUsage = {
  todayTokens: number | null;
  weekTokens: number | null;
  lifetimeTokens: number | null;
  peakDailyTokens: number | null;
};

export type ResetUsage = {
  availableResetCredits: number | null;
  todayAutoReset5h: number;
  weekAutoReset5h: number;
  totalAutoReset5h: number;
  todayAutoResetWeekly: number;
  weekAutoResetWeekly: number;
  totalAutoResetWeekly: number;
  todayManualResetConsumed: number;
  weekManualResetConsumed: number;
  totalManualResetConsumed: number;
  lastAvailableResetCredits: number | null;
};

export type CodexGaugeSnapshot = {
  accountEmail: string | null;
  planType: string | null;
  fiveHour: UsageWindow | null;
  weekly: UsageWindow | null;
  otherWindows: UsageWindow[];
  reset: ResetUsage;
  tokenUsage: TokenUsage;
  credits: unknown | null;
  rateLimitReachedType: string | null;
  lastUpdatedAt: number;
  source: "codex-app-server";
  status: "ok" | "not_logged_in" | "codex_not_found" | "app_server_error" | "partial";
};

export type AppConfig = {
  version: number;
  general: {
    startOnBoot: boolean;
    showOnStartup: boolean;
    alwaysOnTop: boolean;
    lockPosition: boolean;
    opacity: number;
    refreshIntervalSeconds: number;
  };
  codex: {
    command: string;
    transport: string;
    enableUsageRead: boolean;
    enableResetStats: boolean;
  };
  update: {
    autoCheck: boolean;
    channel: string;
  };
  window: {
    x: number | null;
    y: number | null;
    width: number;
    height: number;
  };
};

export type UpdateCheckResult = {
  available: boolean;
  version: string | null;
  message: string;
};
