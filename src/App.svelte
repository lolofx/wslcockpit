<script lang="ts">
  import { onMount } from 'svelte'
  import { startApp } from './state/app.svelte'
  import Shell from './shell/Shell.svelte'
  import Float from './shell/Float.svelte'
  import FeedbackHost from './ui/FeedbackHost.svelte'

  // Fenêtre d'un bloc détaché : l'adresse porte la distro et le bloc (voir Workspace.svelte, detach).
  const float = (() => {
    try {
      const raw = new URLSearchParams(location.search).get('float')
      return raw ? (JSON.parse(raw) as { distro: string; block: import('./workspace/model').Block }) : null
    } catch {
      return null
    }
  })()

  onMount(() => (float ? undefined : startApp()))
</script>

<FeedbackHost />
{#if float}<Float distro={float.distro} block={float.block} />{:else}<Shell />{/if}
