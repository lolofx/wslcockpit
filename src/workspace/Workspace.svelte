<script lang="ts">
  import {
    defaultLayout, parseLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, setBlockProps,
    findBlock, MAX_STACKS, type Layout, type DropTarget,
  } from './model'
  import { registry, def } from './registry'

  let { distro }: { distro: string } = $props()

  const key = (d: string) => `wslcockpit.layout.${d}`
  function load(d: string): Layout {
    try {
      const raw = localStorage.getItem(key(d))
      return raw ? parseLayout(JSON.parse(raw)) : defaultLayout()
    } catch {
      return defaultLayout()
    }
  }

  let layout = $state<Layout>(defaultLayout())
  let loadedFor = '' // évite d'écrire la disposition d'une distro dans la clé d'une autre
  $effect(() => {
    layout = load(distro)
    loadedFor = distro
  })
  $effect(() => {
    const json = JSON.stringify(layout)
    if (loadedFor === distro) localStorage.setItem(key(distro), json)
  })

  let root: HTMLDivElement
  let adding = $state('')

  // --- redimensionnement : poignées entre piles (horizontal) et entre blocs (vertical)
  function startResize(e: PointerEvent, axis: 'x' | 'y', index: number, stackIdx?: number) {
    e.preventDefault()
    const handle = e.currentTarget as HTMLElement
    handle.setPointerCapture(e.pointerId)
    const container = (axis === 'x' ? root : handle.parentElement) as HTMLElement
    const total = axis === 'x' ? container.clientWidth : container.clientHeight
    let last = axis === 'x' ? e.clientX : e.clientY
    const move = (ev: PointerEvent) => {
      const cur = axis === 'x' ? ev.clientX : ev.clientY
      const delta = (cur - last) / total
      last = cur
      if (axis === 'x') layout = { ...layout, stacks: resizeAt(layout.stacks, index, delta) }
      else {
        const s = layout.stacks[stackIdx!]
        const stacks = layout.stacks.slice()
        stacks[stackIdx!] = { ...s, blocks: resizeAt(s.blocks, index, delta) }
        layout = { ...layout, stacks }
      }
    }
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }
  function equalizeAt(axis: 'x' | 'y', stackIdx?: number) {
    if (axis === 'x') layout = { ...layout, stacks: layout.stacks.map((s) => ({ ...s, size: 1 / layout.stacks.length })) }
    else {
      layout = {
        ...layout,
        stacks: layout.stacks.map((s, i) => (i === stackIdx ? { ...s, blocks: s.blocks.map((b) => ({ ...b, size: 1 / s.blocks.length })) } : s)),
      }
    }
  }

  // --- glisser-déposer par événements pointeur (le DnD HTML5 est intercepté par Tauri sous Windows)
  let drag = $state<{ id: string; target: DropTarget | null; ind: { l: number; t: number; w: number; h: number } | null } | null>(null)

  function startDrag(e: PointerEvent, id: string) {
    if ((e.target as HTMLElement).closest('button, details, select, input')) return
    const sx = e.clientX, sy = e.clientY
    const head = e.currentTarget as HTMLElement
    head.setPointerCapture(e.pointerId)
    let active = false
    const move = (ev: PointerEvent) => {
      if (!active && Math.hypot(ev.clientX - sx, ev.clientY - sy) < 6) return
      active = true
      const { target, ind } = computeDrop(ev.clientX, ev.clientY)
      drag = { id, target, ind }
    }
    const up = () => {
      head.removeEventListener('pointermove', move)
      head.removeEventListener('pointerup', up)
      if (active && drag?.target) layout = moveBlock(layout, id, drag.target)
      drag = null
    }
    head.addEventListener('pointermove', move)
    head.addEventListener('pointerup', up)
  }

  function computeDrop(x: number, y: number) {
    const r = root.getBoundingClientRect()
    const rel = (b: DOMRect) => ({ l: b.left - r.left, t: b.top - r.top, w: b.width, h: b.height })
    const edge = 36
    if (layout.stacks.length < MAX_STACKS && (x - r.left < edge || r.right - x < edge)) {
      const left = x - r.left < edge
      return { target: { kind: 'new-stack', at: left ? 0 : layout.stacks.length } as DropTarget, ind: { l: left ? 0 : r.width - 6, t: 0, w: 6, h: r.height } }
    }
    const stackEl = (document.elementsFromPoint(x, y).find((el) => (el as HTMLElement).dataset?.stack) as HTMLElement) ?? null
    if (!stackEl) return { target: null, ind: null }
    const blocks = [...stackEl.querySelectorAll<HTMLElement>(':scope > [data-block]')]
    let index = blocks.length
    for (let i = 0; i < blocks.length; i++) {
      const b = blocks[i].getBoundingClientRect()
      if (y < b.top + b.height / 2) { index = i; break }
    }
    const sb = stackEl.getBoundingClientRect()
    const lineY = index < blocks.length ? blocks[index].getBoundingClientRect().top : (blocks.at(-1)?.getBoundingClientRect().bottom ?? sb.top)
    return {
      target: { kind: 'stack', stackId: stackEl.dataset.stack!, index } as DropTarget,
      ind: { l: sb.left - r.left, t: lineY - r.top - 2, w: sb.width, h: 4 },
    }
  }

  // --- équivalent clavier : menu « Déplacer »
  function shift(id: string, dir: 'left' | 'right' | 'up' | 'down') {
    const pos = findBlock(layout, id)
    if (!pos) return
    const { stack, index } = pos
    if (dir === 'up' && index > 0) layout = moveBlock(layout, id, { kind: 'stack', stackId: layout.stacks[stack].id, index: index - 1 })
    else if (dir === 'down' && index < layout.stacks[stack].blocks.length - 1)
      layout = moveBlock(layout, id, { kind: 'stack', stackId: layout.stacks[stack].id, index: index + 2 })
    else if (dir === 'left' || dir === 'right') {
      const to = stack + (dir === 'left' ? -1 : 1)
      if (to >= 0 && to < layout.stacks.length) layout = moveBlock(layout, id, { kind: 'stack', stackId: layout.stacks[to].id, index: Infinity })
      else if (layout.stacks.length < MAX_STACKS) layout = moveBlock(layout, id, { kind: 'new-stack', at: dir === 'left' ? 0 : layout.stacks.length })
    }
  }

  function add() {
    const [type, i] = adding.split(':')
    const p = def(type)?.presets[Number(i)]
    if (!p) return
    layout = addBlock(layout, makeBlock(type, { ...p.props }))
    adding = ''
  }
  const titleOf = (type: string, props: Record<string, unknown>) => {
    if (type !== 'terminal') return def(type)?.label ?? type
    return props.kind === 'wsl' ? 'Terminal WSL' : props.start === 'profile' ? 'PowerShell (profil)' : 'PowerShell (disque distro)'
  }
</script>

<div class="bar">
  <select bind:value={adding} onchange={add} aria-label="Ajouter un bloc">
    <option value="">+ Ajouter un bloc…</option>
    {#each registry as d}
      {#each d.presets as p, i}<option value="{d.type}:{i}">{p.label}</option>{/each}
    {/each}
  </select>
  <button onclick={() => (layout = defaultLayout())}>Disposition par défaut</button>
</div>

<div class="ws" bind:this={root}>
  {#each layout.stacks as stack, si (stack.id)}
    {#if si > 0}
      <div class="split v" role="separator" aria-orientation="vertical" onpointerdown={(e) => startResize(e, 'x', si - 1)} ondblclick={() => equalizeAt('x')}></div>
    {/if}
    <section class="stack" data-stack={stack.id} style:flex="{stack.size} 1 0">
      {#each stack.blocks as b, bi (b.id)}
        {#if bi > 0}
          <div class="split h" role="separator" aria-orientation="horizontal" onpointerdown={(e) => startResize(e, 'y', bi - 1, si)} ondblclick={() => equalizeAt('y', si)}></div>
        {/if}
        {@const d = def(b.type)}
        <article class="block" data-block class:dragging={drag?.id === b.id} style:flex="{b.size} 1 0">
          <header role="toolbar" tabindex="-1" onpointerdown={(e) => startDrag(e, b.id)}>
            <span class="t">{titleOf(b.type, b.props)}</span>
            <details class="menu">
              <summary aria-label="Menu du bloc">⋯</summary>
              <div class="pop">
                <button onclick={() => shift(b.id, 'left')}>◀ Pile précédente</button>
                <button onclick={() => shift(b.id, 'right')}>▶ Pile suivante</button>
                <button onclick={() => shift(b.id, 'up')}>▲ Monter</button>
                <button onclick={() => shift(b.id, 'down')}>▼ Descendre</button>
                <button onclick={() => (layout = removeBlock(layout, b.id))}>✕ Fermer</button>
              </div>
            </details>
          </header>
          <div class="body">
            {#if d}
              {@const C = d.component}
              <C {distro} block={b} update={(p) => (layout = setBlockProps(layout, b.id, p))} />
            {:else}
              <p class="pad">Bloc inconnu « {b.type} ».</p>
            {/if}
          </div>
        </article>
      {/each}
    </section>
  {/each}
  {#if drag?.ind}
    <div class="ind" style:left="{drag.ind.l}px" style:top="{drag.ind.t}px" style:width="{drag.ind.w}px" style:height="{drag.ind.h}px"></div>
  {/if}
</div>

<style>
  .bar { display: flex; gap: 0.5rem; padding: 0.4rem 0.6rem; border-bottom: 1px solid var(--border); }
  .ws { position: relative; display: flex; flex: 1; min-height: 0; min-width: 0; }
  .stack { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .block { display: flex; flex-direction: column; min-height: 0; border: 1px solid var(--border); border-radius: 6px; margin: 2px; overflow: hidden; }
  .block.dragging { opacity: 0.4; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 0.2rem 0.5rem; background: var(--surface-2); cursor: grab; user-select: none; touch-action: none; }
  .t { font-size: 0.85rem; font-weight: 600; }
  .body { flex: 1; min-height: 0; overflow: auto; }
  .split { flex: none; touch-action: none; }
  .split.v { width: 6px; cursor: col-resize; }
  .split.h { height: 6px; cursor: row-resize; }
  .split:hover { background: var(--surface-3); }
  .ind { position: absolute; background: #3b82f6; border-radius: 2px; pointer-events: none; opacity: 0.8; z-index: 5; }
  .menu { position: relative; }
  .menu summary { cursor: pointer; list-style: none; padding: 0 0.4rem; }
  .pop { position: absolute; right: 0; z-index: 10; display: grid; background: var(--surface-1); border: 1px solid var(--surface-3); border-radius: 6px; padding: 0.25rem; min-width: 11rem; }
  .pop button { text-align: left; border: 0; background: none; padding: 0.3rem 0.5rem; color: inherit; cursor: pointer; }
  .pop button:hover { background: color-mix(in srgb, currentColor 12%, transparent); }
  @media (max-width: 700px) {
    .ws { flex-direction: column; overflow: auto; }
    .stack { flex: none !important; }
    .block { flex: none !important; min-height: 14rem; }
    .split.v { display: none; }
  }
</style>
