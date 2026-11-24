<script lang="ts">
  import { onMount } from 'svelte'
  import { filterCommands, type Command } from './filter'

  let { commands, onclose }: { commands: Command[]; onclose: () => void } = $props()
  let q = $state('')
  let sel = $state(0)
  let input: HTMLInputElement
  const shown = $derived(filterCommands(commands, q).slice(0, 12))
  $effect(() => { q; sel = 0 })
  onMount(() => input.focus())

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose()
    else if (e.key === 'ArrowDown') { e.preventDefault(); sel = (sel + 1) % Math.max(shown.length, 1) }
    else if (e.key === 'ArrowUp') { e.preventDefault(); sel = (sel - 1 + shown.length) % Math.max(shown.length, 1) }
    else if (e.key === 'Enter' && shown[sel]) { const c = shown[sel]; onclose(); c.run() }
  }
</script>

<div class="overlay" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="pal" role="dialog" aria-modal="true" aria-label="Palette de commandes">
    <input bind:this={input} bind:value={q} onkeydown={key} placeholder="Tape une action ou une distribution…" role="combobox" aria-expanded="true" aria-controls="pal-list" />
    <ul id="pal-list" role="listbox">
      {#each shown as c, i (c.id)}
        <li role="option" aria-selected={i === sel}>
          <button class:sel={i === sel} onclick={() => { onclose(); c.run() }} onmousemove={() => (sel = i)}>{c.label}</button>
        </li>
      {:else}
        <li class="none">Aucune commande.</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 150; background: var(--overlay); display: flex; justify-content: center; align-items: flex-start; padding: 12vh var(--sp-4) 0; }
  .pal { width: min(560px, 100%); background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-lg); box-shadow: var(--shadow); overflow: hidden; }
  input { width: 100%; box-sizing: border-box; font: inherit; padding: var(--sp-3) var(--sp-4); border: 0; border-bottom: 1px solid var(--border); background: transparent; color: var(--text); outline: none; }
  ul { list-style: none; margin: 0; padding: var(--sp-1); max-height: 50vh; overflow: auto; }
  li button { width: 100%; text-align: left; font: inherit; font-size: var(--fs-sm); padding: var(--sp-2) var(--sp-3); border: 0; border-radius: var(--radius-sm); background: none; color: var(--text); cursor: pointer; }
  li button.sel { background: var(--surface-3); }
  .none { padding: var(--sp-3); color: var(--text-dim); font-size: var(--fs-sm); }
</style>
