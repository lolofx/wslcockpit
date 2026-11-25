<script lang="ts">
  // Fenêtre d'un bloc détaché. À sa fermeture (croix ou « Rattacher »), le bloc retourne dans la fenêtre principale.
  import { onMount } from 'svelte'
  import { emit } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { startApp } from '../state/app.svelte'
  import { def } from '../workspace/registry'
  import type { Block } from '../workspace/model'
  import Button from '../ui/Button.svelte'

  let { distro, block: initial }: { distro: string; block: Block } = $props()
  let block = $state<Block>(initial)

  onMount(() => {
    const stop = startApp(true)
    const win = getCurrentWindow()
    const un = win.onCloseRequested(() => {
      void emit('block-reattach', { distro, block: $state.snapshot(block) })
    })
    return () => {
      stop()
      void un.then((f) => f())
    }
  })

  const d = $derived(def(block.type))
</script>

<div class="float">
  <header>
    <span>{d?.label ?? block.type} · {distro}</span>
    <Button onclick={() => getCurrentWindow().close()}>Rattacher</Button>
  </header>
  <div class="body">
    {#if d}
      {@const C = d.component}
      <C {distro} {block} update={(p) => (block = { ...block, props: { ...block.props, ...p } })} />
    {/if}
  </div>
</div>

<style>
  .float { height: 100vh; display: flex; flex-direction: column; }
  header { display: flex; justify-content: space-between; align-items: center; padding: var(--sp-1) var(--sp-3); background: var(--surface-2); border-bottom: 1px solid var(--border); font-size: var(--fs-sm); font-weight: 600; }
  .body { flex: 1; min-height: 0; overflow: auto; container-type: inline-size; }
</style>
