<script lang="ts">
  import { formatPercent } from "../lib/format";
  import { ringTone, ringValue } from "../lib/usage";
  import type { UsageWindow } from "../lib/types";

  export let window: UsageWindow | null = null;
  export let label = "";

  $: value = ringValue(window);
  $: tone = ringTone(window?.usedPercent);
  $: offset = 100 - value;
</script>

<div class={`ring tone-${tone}`}>
  <svg viewBox="0 0 36 36" aria-hidden="true">
    <path class="track" d="M18 2.8a15.2 15.2 0 1 1 0 30.4a15.2 15.2 0 1 1 0 -30.4" />
    <path
      class="value"
      pathLength="100"
      stroke-dasharray="100"
      stroke-dashoffset={offset}
      d="M18 2.8a15.2 15.2 0 1 1 0 30.4a15.2 15.2 0 1 1 0 -30.4"
    />
  </svg>
  <div>
    <span>{label}</span>
    <strong>{formatPercent(window?.usedPercent)}</strong>
  </div>
</div>
