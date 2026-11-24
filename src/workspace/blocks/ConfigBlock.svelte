<script lang="ts">
  import type { BlockContext } from '../registry'
  let { distro }: BlockContext = $props()
</script>

<div class="pad"><p>Formulaires wsl.conf de {distro} (prototype : le ConfigPanel existant sera branché au lot 6.3).</p></div>

<style>
  .pad { padding: 0.75rem; }
</style>
