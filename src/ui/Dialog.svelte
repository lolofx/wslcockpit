<script lang="ts">
  import type { Snippet } from 'svelte'
  import { onMount } from 'svelte'

  let { title, onclose, width = 520, children, footer }: {
    title: string
    onclose: () => void
    width?: number
    children: Snippet
    footer?: Snippet
  } = $props()

  let el: HTMLDivElement
  let previous: Element | null = null

  onMount(() => {
    previous = document.activeElement
    const first = el.querySelector<HTMLElement>('input, select, textarea') ?? el.querySelector<HTMLElement>('button')
    first?.focus()
    return () => (previous as HTMLElement | null)?.focus?.()
  })

  // Échap ferme ; Tab reste dans la boîte.
  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.stopPropagation(); onclose(); return }
    if (e.key !== 'Tab') return
    const f = [...el.querySelectorAll<HTMLElement>('button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])')].filter((x) => !x.hasAttribute('disabled'))
    if (!f.length) return
    const a = f[0], z = f[f.length - 1]
    if (e.shiftKey && document.activeElement === a) { e.preventDefault(); z.focus() }
    else if (!e.shiftKey && document.activeElement === z) { e.preventDefault(); a.focus() }
  }
</script>

<div class="overlay" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" bind:this={el} onkeydown={key} style:width="min({width}px, 100%)">
    <h2>{title}</h2>
    <div class="body">{@render children()}</div>
    {#if footer}<div class="foot">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 100; background: var(--overlay); display: flex; align-items: center; justify-content: center; padding: var(--sp-4); animation: fade-in 120ms ease; }
  .dialog { animation: pop-in 150ms var(--ease); }
  .dialog { max-height: 100%; overflow: auto; background: var(--surface-1); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius-lg); box-shadow: var(--shadow); padding: var(--sp-5); display: grid; gap: var(--sp-4); }
  h2 { margin: 0; font-size: var(--fs-lg); overflow-wrap: anywhere; }
  .body { display: grid; gap: var(--sp-3); }
  .foot { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: var(--sp-2); }
</style>
