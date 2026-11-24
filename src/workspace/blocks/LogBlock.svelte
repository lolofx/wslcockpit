<script lang="ts">
  import type { BlockContext } from '../registry'
  let { distro }: BlockContext = $props()
</script>

<div class="pad"><p>Journal de {distro} (prototype, vide).</p></div>

<style>
  .pad { padding: 0.75rem; }
</style>
