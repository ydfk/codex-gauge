export type UsageWindow = {
  name: "5h" | "weekly";
  usedPercent?: number | null;
  remainingPercent?: number | null;
  resetAt?: number | null;
  windowDurationSeconds?: number | null;
};

export type UsageCredits = {
  remaining?: number | null;
  availableCount?: number | null;
  resetCredits?: number | null;
  resetAt?: number | null;
  items?: ResetCreditItem[];
};

export type ResetCreditItem = {
  status?: string | null;
  title?: string | null;
  grantedAt?: string | null;
  expiresAt?: string | null;
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

export type CodexUsageSnapshot = {
  source: "auth-json" | "app-server" | "session-log";
  status: "ok" | "not_logged_in" | "invalid_auth" | "request_failed";
  planType?: string | null;
  primaryWindow?: UsageWindow | null;
  primaryWindowUnlimited: boolean;
  secondaryWindow?: UsageWindow | null;
  credits?: UsageCredits | null;
  rateLimitReachedType?: string | null;
  updatedAt: number;
};

export type AppConfig = {
  version: number;
  general: {
    startOnBoot: boolean;
    showOnStartup: boolean;
    alwaysOnTop: boolean;
    mainAlwaysOnTop: boolean;
    topAlwaysOnTop: boolean;
    lockPosition: boolean;
    topLockPosition: boolean;
    oledShiftEnabled: boolean;
    topStatusEnabled: boolean;
    opacity: number;
    refreshIntervalSeconds: number;
  };
  codex: {
    command: string;
    transport: string;
    preferredProvider: "api" | "app-server";
    enableUsageRead: boolean;
    enableResetStats: boolean;
  };
  update: {
    autoCheck: boolean;
    channel: string;
    endpoint: string;
  };
  window: {
    x: number | null;
    y: number | null;
    width: number;
    height: number;
  };
  macos: {
    menuBarDisplay: "fiveHour" | "iconOnly" | "fiveAndSeven";
  };
};

export type UpdateCheckResult = {
  available: boolean;
  version: string | null;
  message: string;
};
