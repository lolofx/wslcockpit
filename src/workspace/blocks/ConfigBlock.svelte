<script lang="ts">
  import type { BlockContext } from '../registry'
  import ConfigPanel from '../../lib/ConfigPanel.svelte'
  let { distro }: BlockContext = $props()
</script>

<div class="pad">
  {#key distro}<ConfigPanel distros={[]} fixed={distro} />{/key}
</div>

<style>
  .pad { padding: var(--sp-3); }
  .pad :global(.row) { grid-template-columns: minmax(8rem, 14rem) 1fr; }
  @container (max-width: 30rem) { .pad :global(.row) { grid-template-columns: 1fr; } }
</style>
