<script lang="ts">
  import { onMount } from 'svelte'
  import { startApp } from './state/app.svelte'
  import Shell from './shell/Shell.svelte'
  import FeedbackHost from './ui/FeedbackHost.svelte'

  onMount(() => startApp())
</script>

<FeedbackHost />
<Shell />
