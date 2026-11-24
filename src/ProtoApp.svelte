<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import Workspace from './workspace/Workspace.svelte'

  let { onexit }: { onexit: () => void } = $props()
  let names = $state<string[]>([])
  let current = $state('')
  let drawer = $state(false)

  onMount(async () => {
    try {
      const l = await invoke<{ name: string }[]>('distros')
      names = l.map((d) => d.name)
    } catch {
      names = ['Ubuntu', 'Debian'] // aperçu hors Tauri
    }
    current = names[0] ?? ''
  })
</script>

<div class="app">
  <nav class:open={drawer}>
    <button class="hamb" onclick={() => (drawer = !drawer)} aria-label="Distributions">☰</button>
    <div class="items">
      {#each names as n}
        <button class="d" class:on={n === current} title={n} onclick={() => { current = n; drawer = false }}>{n.slice(0, 2).toUpperCase()}<span>{n}</span></button>
      {/each}
    </div>
    <button class="d exit" title="Quitter le prototype" onclick={onexit}>↩<span>Retour</span></button>
  </nav>
  <main>
    {#if current}<Workspace distro={current} />{:else}<p>Aucune distribution.</p>{/if}
  </main>
</div>

<style>
  .app { display: flex; height: 100vh; }
  nav { display: flex; flex-direction: column; width: 3.2rem; padding: 0.4rem 0.25rem; gap: 0.3rem; border-right: 1px solid color-mix(in srgb, currentColor 15%, transparent); }
  .items { display: flex; flex-direction: column; gap: 0.3rem; flex: 1; }
  .d { height: 2.4rem; border-radius: 8px; border: 1px solid transparent; background: color-mix(in srgb, currentColor 8%, transparent); color: inherit; cursor: pointer; font-weight: 600; font-size: 0.8rem; }
  .d span { display: none; }
  .d.on { border-color: #3b82f6; }
  .hamb { display: none; }
  main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  @media (max-width: 700px) {
    .app { flex-direction: column; height: auto; min-height: 100vh; }
    nav { width: auto; flex-direction: row; border-right: 0; border-bottom: 1px solid color-mix(in srgb, currentColor 15%, transparent); flex-wrap: wrap; }
    .hamb { display: block; }
    .items { display: none; flex-direction: row; flex-wrap: wrap; }
    nav.open .items { display: flex; }
    .d { height: 2rem; padding: 0 0.6rem; }
    .d span { display: inline; margin-left: 0.4rem; font-weight: 400; }
  }
</style>
