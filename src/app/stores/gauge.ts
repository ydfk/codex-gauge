import { writable } from "svelte/store";
import type { AppConfig, CodexGaugeSnapshot } from "../../lib/types";

export const snapshot = writable<CodexGaugeSnapshot | null>(null);
export const config = writable<AppConfig | null>(null);
export const busy = writable(false);
export const message = writable("");
