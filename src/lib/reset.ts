import type { ResetUsage } from "./types";

export function resetCreditText(reset: ResetUsage | null | undefined) {
  const count = reset?.availableResetCredits;
  return count == null ? "R未知" : `R${count}`;
}
