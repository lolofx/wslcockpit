<script lang="ts">
  import type { BlockContext } from '../registry'
  let { distro }: BlockContext = $props()
</script>

<div class="pad"><b>{distro}</b><p>Prototype : l'état réel (politique, VHDX, disque) viendra du composant existant.</p></div>

<style>
  .pad { padding: 0.75rem; }
</style>
