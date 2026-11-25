<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { MenuItem } from '../shell/menus'


  let { items, label = 'Actions', trigger }: { items: MenuItem[]; label?: string; trigger?: Snippet } = $props()

  let open = $state(false)
  let root: HTMLElement
  let list = $state<HTMLElement>()

  let pos = $state({ left: 0, top: 0 })
  // Popover en position fixe, replacé dans la fenêtre (retourné vers le haut ou la gauche si besoin).
  function place() {
    const t = root.querySelector('.trigger')!.getBoundingClientRect()
    const r = list!.getBoundingClientRect()
    const m = 8
    let left = t.right - r.width
    if (left + r.width > innerWidth - m) left = innerWidth - m - r.width
    if (left < m) left = m
    let top = t.bottom + 4
    if (top + r.height > innerHeight - m) top = Math.max(m, t.top - 4 - r.height)
    pos = { left, top }
  }
  function toggle() {
    open = !open
    if (open) queueMicrotask(() => { place(); focusItem(0) })
  }
  const buttons = () => [...(list?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])]
  function focusItem(i: number) { const b = buttons(); b[(i + b.length) % b.length]?.focus() }
  function key(e: KeyboardEvent) {
    if (!open) return
    const b = buttons(), i = b.indexOf(document.activeElement as HTMLButtonElement)
    if (e.key === 'Escape') { open = false; (root.querySelector('.trigger') as HTMLElement).focus() }
    else if (e.key === 'ArrowDown') { e.preventDefault(); focusItem(i + 1) }
    else if (e.key === 'ArrowUp') { e.preventDefault(); focusItem(i - 1) }
  }
  function choose(it: MenuItem) { open = false; it.onselect() }
</script>

<svelte:window onresize={() => open && (open = false)} onpointerdown={(e) => open && !root.contains(e.target as Node) && (open = false)} />

<div class="menu" bind:this={root} onkeydown={key} role="presentation">
  <button class="trigger" aria-haspopup="menu" aria-expanded={open} aria-label={label} onclick={toggle}>
    {#if trigger}{@render trigger()}{:else}⋯{/if}
  </button>
  {#if open}
    <div class="pop" role="menu" bind:this={list} style="left:{pos.left}px;top:{pos.top}px">
      {#each items as it}
        <button role="menuitem" class:danger={it.danger} disabled={it.disabled} onclick={() => choose(it)}>{it.label}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu { position: relative; display: inline-block; }
  .trigger { font: inherit; padding: var(--sp-1) var(--sp-2); border-radius: var(--radius); border: 1px solid transparent; background: transparent; color: var(--text); cursor: pointer; }
  .trigger:hover { background: var(--surface-2); }
  .trigger:focus-visible { outline: none; box-shadow: var(--focus); }
  .pop { position: fixed; z-index: 50; max-height: calc(100vh - 16px); overflow-y: auto; min-width: 12rem; display: grid; padding: var(--sp-1); background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); }
  .pop button { font: inherit; font-size: var(--fs-sm); text-align: left; padding: var(--sp-2) var(--sp-3); border: 0; border-radius: var(--radius-sm); background: none; color: var(--text); cursor: pointer; }
  .pop button:hover:not(:disabled), .pop button:focus-visible { background: var(--surface-2); outline: none; }
  .pop button:disabled { opacity: 0.45; cursor: default; }
  .pop .danger { color: var(--danger); }
</style>
