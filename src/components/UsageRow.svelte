<script lang="ts">
  import { formatDateTime, formatPercent, formatReset, windowTitle } from "../lib/format";
  import type { UsageWindow } from "../lib/types";

  export let window: UsageWindow | null = null;
</script>

<div class="usage-row">
  <div>
    <span>{windowTitle(window)}</span>
    <strong>已用 {formatPercent(window?.usedPercent)}</strong>
  </div>
  <div>
    <span>剩余 {formatPercent(window?.remainingPercent)}</span>
    <strong>{formatReset(window?.resetRemainingText)}</strong>
  </div>
  <div>
    <span>重置</span>
    <strong>{formatDateTime(window?.resetsAt)}</strong>
  </div>
</div>
