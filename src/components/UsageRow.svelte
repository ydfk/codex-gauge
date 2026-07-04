<script lang="ts">
  import { formatDateTime, formatPercent, formatReset, windowTitle } from "../lib/format";
  import type { UsageWindow } from "../lib/types";

  export let window: UsageWindow | null = null;
</script>

<div class="usage-row">
  <div>
    <span>{windowTitle(window)}</span>
    <strong>使用 {formatPercent(window?.usedPercent)}</strong>
  </div>
  <div>
    <span>剩余 {formatPercent(window?.remainingPercent)}</span>
    <strong>{formatReset(window?.resetAt)}</strong>
  </div>
  <div>
    <span>重置</span>
    <strong>{formatDateTime(window?.resetAt)}</strong>
  </div>
</div>
