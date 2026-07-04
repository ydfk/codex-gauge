import { writable } from "svelte/store";
import type { AppConfig, CodexUsageSnapshot } from "../../lib/types";

export const snapshot = writable<CodexUsageSnapshot | null>(null);
export const config = writable<AppConfig | null>(null);
export const busy = writable(false);
export const message = writable("");
