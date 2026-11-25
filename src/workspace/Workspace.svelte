<script lang="ts">
  import { untrack } from 'svelte'
  import { flip } from 'svelte/animate'
  import { scale } from 'svelte/transition'
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import {
    defaultLayout, emptyLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, setBlockProps,
    findBlock, dockBlock, undockBlock, undockTo, newStackSlot, MAX_STACKS, blockDistro, type ProjectRoot, type Layout, type DropTarget, type DragTarget, type Block,
  } from './model'
  import { registry, def } from './registry'
  import { app, display } from '../state/app.svelte'
  import Icon from '../icons/Icon.svelte'
  import { launchers } from '../state/launchers.svelte'
  import { availableIn, launchProps } from '../launchers/model'
  import { layouts, layoutFor, setLayout, saveAsPreset, removePreset, presetLayout } from '../state/layouts.svelte'
  import { promptText, confirmAction } from '../ui/feedback.svelte'
  import Menu from '../ui/Menu.svelte'
  import { t } from '../i18n/index.svelte'
  import type { MenuItem } from '../shell/menus'

  // `space` : clé de la disposition (par défaut la distro) ; dans un affichage, `distro` est vide et chaque bloc porte la sienne.
  let { distro, space = distro, views: inView = false, projectRoot }: { distro: string; space?: string; views?: boolean; projectRoot?: ProjectRoot } = $props()

  // La disposition vit dans le magasin (layouts.json côté Rust) ; ici une copie de travail réactive.
  let layout = $state<Layout>(defaultLayout())
  let loadedFor = '' // évite d'écrire la disposition d'une distro dans celle d'une autre
  $effect(() => {
    const d = space
    void layouts.rev
    if (!layouts.loaded) return
    untrack(() => {
      layout = layoutFor(d)
      loadedFor = d
    })
  })
  $effect(() => {
    const snap = $state.snapshot(layout)
    untrack(() => {
      if (loadedFor === space) setLayout(space, snap)
    })
  })

  // Un terminal vit tant que son bloc existe (déplacer ou détacher ne le coupe pas) : quand un bloc
  // terminal disparaît de la disposition de la même distro, sa session est fermée.
  const keep = new Set<string>() // blocs détachés : leur session continue dans la fenêtre flottante
  let prevTerms = new Set<string>()
  let prevFor = ''
  $effect(() => {
    const snap = $state.snapshot(layout)
    const ids = new Set([...snap.stacks.flatMap((s) => s.blocks), ...(snap.dock ?? [])].filter((b) => b.type === 'terminal').map((b) => b.id))
    untrack(() => {
      if (prevFor === loadedFor) {
        for (const k of prevTerms) if (!ids.has(k) && !keep.delete(k)) void invoke('pty_close', { id: k }).catch(() => {})
      }
      for (const k of ids) keep.delete(k) // rattaché : il redevient un bloc ordinaire
      prevTerms = ids
      prevFor = loadedFor
    })
  })

  let root: HTMLDivElement

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
  let swallowClick = false
  let dragStart = $state<{ x: number; y: number } | null>(null)
  let railEl = $state<HTMLElement | null>(null)
  let drag = $state<{ id: string; x: number; y: number; target: DragTarget | null; from: 'stack' | 'dock'; ind: { l: number; t: number; w: number; h: number } | null } | null>(null)

  function startDrag(e: PointerEvent, id: string, from: 'stack' | 'dock' = 'stack') {
    if (from === 'stack' && (e.target as HTMLElement).closest('button, details, select, input')) return
    const sx = e.clientX, sy = e.clientY
    dragStart = { x: sx, y: sy }
    const head = e.currentTarget as HTMLElement
    head.setPointerCapture(e.pointerId)
    let active = false
    const move = (ev: PointerEvent) => {
      if (!active && Math.hypot(ev.clientX - sx, ev.clientY - sy) < 6) return
      active = true
      const { target, ind } = computeDrop(ev.clientX, ev.clientY, from)
      drag = { id, x: ev.clientX, y: ev.clientY, target, from, ind }
    }
    const up = () => {
      head.removeEventListener('pointermove', move)
      head.removeEventListener('pointerup', up)
      const tg = drag?.target
      if (active && tg) {
        if (tg.kind === 'dock') layout = from === 'stack' ? dockBlock(layout, id) : layout
        else layout = from === 'dock' ? undockTo(layout, id, tg) : moveBlock(layout, id, tg)
      }
      drag = null
      dragStart = null
      if (active) { swallowClick = true; setTimeout(() => (swallowClick = false), 0) }
    }
    head.addEventListener('pointermove', move)
    head.addEventListener('pointerup', up)
  }

  function computeDrop(x: number, y: number, from: 'stack' | 'dock'): { target: DragTarget | null; ind: { l: number; t: number; w: number; h: number } | null } {
    const r = root.getBoundingClientRect()
    const rel = (b: DOMRect) => ({ l: b.left - r.left, t: b.top - r.top, w: b.width, h: b.height })
    const edge = 36
    // La barre des blocs en attente : déposer dessus met le bloc de côté.
    const rail = railEl?.getBoundingClientRect()
    if (rail && from === 'stack' && x >= rail.left - 8 && x <= rail.right + 8 && y >= rail.top && y <= rail.bottom)
      return { target: { kind: 'dock' } as DragTarget, ind: null }
    // Nouvelle pile : aux deux bords et au milieu de chaque séparation entre deux piles.
    if (layout.stacks.length < MAX_STACKS) {
      const rects = [...root.querySelectorAll<HTMLElement>(':scope > [data-stack]')].map((el) => el.getBoundingClientRect())
      const at = newStackSlot(rects, x, edge)
      if (at !== null) {
        const cx = at === 0 ? r.left : at === rects.length ? r.right : (rects[at - 1].right + rects[at].left) / 2
        const left = at === 0 ? 0 : at === rects.length ? r.width - 6 : cx - r.left - 3
        return { target: { kind: 'new-stack', at } as DragTarget, ind: { l: left, t: 0, w: 6, h: r.height } }
      }
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

  function addPreset(type: string, i: number, onDistro?: string) {
    const p = def(type)?.presets[i]
    if (p) layout = addBlock(layout, makeBlock(type, { ...p.props, ...(onDistro ? { distro: onDistro } : {}) }))
  }
  // Un bloc en ouvre un autre (ex. fichier .md du bloc Projet) : on réutilise un bloc du même type s'il existe.
  function openBlock(type: string, props: Record<string, unknown>) {
    const ex = layout.stacks.flatMap((s) => s.blocks).find((x) => x.type === type)
    layout = ex ? setBlockProps(layout, ex.id, props) : addBlock(layout, makeBlock(type, props))
  }
  // Un bloc a besoin d'une distro, sauf PowerShell sur le profil Windows.
  const needsDistro = (type: string, props: Record<string, unknown>) =>
    type !== 'project' && type !== 'markdown' && !(type === 'terminal' && props.kind === 'powershell' && props.start === 'profile')
  const baseTitle = (type: string, props: Record<string, unknown>) => {
    if (type !== 'terminal') return def(type)?.label ?? type
    if (typeof props.title === 'string' && props.title) return props.title
    return props.kind === 'wsl' ? t('workspace.title.wsl') : props.start === 'profile' ? t('workspace.title.psProfile') : t('workspace.title.psDistro')
  }
  // Dans un affichage, le titre rappelle la distro du bloc.
  const titleOf = (type: string, props: Record<string, unknown>) => {
    const b = baseTitle(type, props)
    return inView && typeof props.distro === 'string' && props.distro ? `${b} · ${display(props.distro)}` : b
  }

  // Lanceurs : un terminal qui exécute la commande, dans le dossier du projet de l'affichage s'il y en a un.
  const launcherItems = $derived<MenuItem[]>(
    launchers.doc.launchers
      .filter((l) => availableIn(l, inView ? null : distro))
      .flatMap((l) => {
        const targets = l.target === 'windows' ? [''] : l.distro ? [l.distro] : inView ? app.distros.map((x) => x.name) : [distro]
        return targets.map((tg) => ({
          label: `▶ ${l.name}${l.target === 'wsl' && !l.distro && inView ? ' · ' + display(tg) : ''}`,
          onselect: () => (layout = addBlock(layout, makeBlock('terminal', launchProps(l, tg, projectRoot)))),
        }))
      }),
  )
  const addItems = $derived<MenuItem[]>([...launcherItems, ...registry.flatMap((d) =>
      d.presets.flatMap((p, i) =>
        !inView || !needsDistro(d.type, p.props)
          ? [{ label: p.label, onselect: () => addPreset(d.type, i) }]
          : app.distros.map((x) => ({ label: `${p.label} · ${display(x.name)}`, onselect: () => addPreset(d.type, i, x.name) })),
      ),
    )])

  const layoutItems = $derived<MenuItem[]>([
    { label: t('workspace.layout.save'), onselect: () => void savePresetAsk() },
    ...Object.keys(layouts.doc.presets).flatMap((n) => [
      { label: t('workspace.layout.apply', { name: n }), onselect: () => { const l = presetLayout(n); if (l) layout = l } },
      { label: t('workspace.layout.delete', { name: n }), danger: true, onselect: () => void deletePresetAsk(n) },
    ]),
    { label: inView ? t('workspace.layout.clear') : t('workspace.layout.reset'), onselect: () => (layout = inView ? emptyLayout() : defaultLayout()) },
  ])

  async function savePresetAsk() {
    const n = await promptText({ title: t('workspace.layout.nameTitle'), message: t('workspace.layout.nameHint') })
    if (n?.trim()) saveAsPreset(n, $state.snapshot(layout))
  }
  async function deletePresetAsk(n: string) {
    if (await confirmAction({ title: t('workspace.layout.deleteConfirm', { name: n }), confirmLabel: t('workspace.layout.deleteLabel'), danger: true })) removePreset(n)
  }

  // Fenêtre flottante : le bloc quitte la disposition et vit dans sa propre fenêtre ; à sa fermeture
  // la fenêtre émet « block-reattach » et le bloc revient (voir state/app.svelte.ts).
  function detach(b: Block) {
    const bd = blockDistro(b, distro)
    const payload = encodeURIComponent(JSON.stringify({ distro: bd, space, block: $state.snapshot(b) }))
    const w = new WebviewWindow(`float-${b.id}`, {
      url: `index.html?float=${payload}`,
      title: `${baseTitle(b.type, b.props)} · ${bd}`,
      width: 640,
      height: 420,
      minWidth: 320,
      minHeight: 200,
    })
    w.once('tauri://error', () => {}) // échec de création : le bloc reste en place (retiré seulement au succès)
    w.once('tauri://created', () => {
      keep.add(b.id)
      layout = removeBlock(layout, b.id)
    })
  }

  const blockMenu = (b: Block): MenuItem[] => [
    { label: t('workspace.block.prevStack'), onselect: () => shift(b.id, 'left') },
    { label: t('workspace.block.nextStack'), onselect: () => shift(b.id, 'right') },
    { label: t('workspace.block.up'), onselect: () => shift(b.id, 'up') },
    { label: t('workspace.block.down'), onselect: () => shift(b.id, 'down') },
    { label: t('workspace.block.dock'), onselect: () => (layout = dockBlock(layout, b.id)) },
    { label: t('workspace.block.detach'), onselect: () => detach(b) },
    { label: t('workspace.block.close'), danger: true, onselect: () => (layout = removeBlock(layout, b.id)) },
  ]
</script>

<div class="bar">
  <Menu items={addItems} label={t('workspace.add.label')}>{#snippet trigger()}{t('workspace.add.trigger')}{/snippet}</Menu>
  <Menu items={layoutItems} label={t('workspace.layout.label')}>{#snippet trigger()}{t('workspace.layout.label')}{/snippet}</Menu>
</div>

<div class="area">
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
            <Menu items={blockMenu(b)} label={t('workspace.block.menu')} />
          </header>
          <div class="body">
            {#if d}
              {@const C = d.component}
              {#if needsDistro(b.type, b.props) && app.loaded && !app.distros.some((x) => x.name === blockDistro(b, distro))}
                <p class="pad">{t('workspace.block.distroMissing', { name: blockDistro(b, distro) || '?' })}</p>
              {:else}
              <C distro={blockDistro(b, distro)} root={projectRoot} open={openBlock} block={b} update={(p) => (layout = setBlockProps(layout, b.id, p))} />
              {/if}
            {:else}
              <p class="pad">{t('workspace.block.unknown', { type: b.type })}</p>
            {/if}
          </div>
        </article>
      {/each}
    </section>
  {/each}
  {#if layout.stacks.length === 0 && !layout.dock?.length}
    <p class="empty">{t('workspace.empty')}</p>
  {/if}
  {#if drag?.ind}
    <div class="ind" style:left="{drag.ind.l}px" style:top="{drag.ind.t}px" style:width="{drag.ind.w}px" style:height="{drag.ind.h}px"></div>
  {/if}
</div>
  <aside class="dock" class:empty-zone={!layout.dock?.length} class:armed={drag?.from === 'stack'} class:active={drag?.target?.kind === 'dock'} bind:this={railEl} aria-label={t('workspace.dock.label')} title={layout.dock?.length ? undefined : t('workspace.dock.dropHint')}>
    {#each layout.dock ?? [] as b (b.id)}
      <button class="dk" class:dragged={drag?.id === b.id && drag.from === 'dock'} style:transform={drag?.id === b.id && drag.from === 'dock' && dragStart ? `translate(${drag.x - dragStart.x}px, ${drag.y - dragStart.y}px)` : undefined}
        animate:flip={{ duration: 150 }} in:scale={{ duration: 150, start: 0.6 }} out:scale={{ duration: 120, start: 0.6 }} title={t('workspace.dock.restoreTitle', { title: titleOf(b.type, b.props) })} aria-label={t('workspace.dock.restore', { title: titleOf(b.type, b.props) })}
        onpointerdown={(e) => startDrag(e, b.id, 'dock')} onclick={() => { if (!swallowClick) layout = undockBlock(layout, b.id) }}>
        <Icon spec={{ kind: 'symbol', value: def(b.type)?.icon ?? 'box' }} size={18} />
      </button>
    {/each}
    {#if !layout.dock?.length && drag?.from === 'stack'}<Icon spec={{ kind: 'symbol', value: 'box' }} size={20} />{/if}
  </aside>
</div>

{#if drag}
  {@const gb = layout.stacks.flatMap((s) => s.blocks).find((x) => x.id === drag!.id)}
  {#if gb}
    <div class="ghost" style:left="{drag.x + 12}px" style:top="{drag.y + 12}px">{titleOf(gb.type, gb.props)}</div>
  {/if}
{/if}

<style>
  .bar { display: flex; gap: var(--sp-2); padding: var(--sp-1) var(--sp-3); border-bottom: 1px solid var(--border); background: var(--surface-1); }
  .pad { padding: var(--sp-3); color: var(--text-dim); }
  .empty { margin: auto; color: var(--text-dim); text-align: center; padding: var(--sp-4); }
  .area { display: flex; flex: 1; min-height: 0; min-width: 0; }
  .dock { flex: none; display: flex; flex-direction: column; align-items: center; gap: var(--sp-1); width: 2.9rem; box-sizing: border-box; margin: 2px 2px 2px 0; padding: var(--sp-2) 0; border: 1px solid var(--border); border-radius: var(--radius-lg); background: var(--surface-2); color: var(--text-dim); overflow-y: auto; transition: border-color var(--dur), background var(--dur), color var(--dur); }
  .dock.empty-zone { justify-content: center; border: 2px dashed transparent; background: transparent; }
  .dock.empty-zone.armed { border-color: var(--border); }
  .dock.active { border-color: var(--accent); color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, var(--surface-2)); }
  .dk { display: grid; place-items: center; width: 2.2rem; height: 2.2rem; flex: none; border: 1px solid transparent; border-radius: var(--radius); background: transparent; color: var(--text); cursor: grab; touch-action: none; transition: background var(--dur), border-color var(--dur); }
  .dk:hover, .dk:focus-visible { background: var(--surface-3); border-color: var(--accent); outline: none; }
  .dk.dragged { position: relative; z-index: 6; pointer-events: none; cursor: grabbing; background: var(--surface-3); border-color: var(--accent); box-shadow: var(--shadow); transition: none; }
  .ws { position: relative; display: flex; flex: 1; min-height: 0; min-width: 0; padding: 2px; }
  .stack { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .block { display: flex; flex-direction: column; min-height: 0; border: 1px solid var(--border); border-radius: var(--radius-lg); transition: opacity var(--dur), border-color var(--dur); margin: 2px; background: var(--surface-1); }
  .block.dragging { opacity: 0.35; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--sp-2) 0 var(--sp-3); background: var(--surface-2); border-radius: var(--radius-lg) var(--radius-lg) 0 0; cursor: grab; user-select: none; touch-action: none; }
  .t { font-size: var(--fs-sm); font-weight: 600; }
  .body { flex: 1; min-height: 0; overflow: auto; container-type: inline-size; border-radius: 0 0 var(--radius-lg) var(--radius-lg); }
  .split { flex: none; touch-action: none; }
  .split.v { width: 6px; cursor: col-resize; }
  .split.h { height: 6px; cursor: row-resize; }
  .split:hover { background: var(--surface-3); }
  .ind { position: absolute; transition: left 120ms ease, top 120ms ease, width 120ms ease, height 120ms ease; background: var(--accent); border-radius: 2px; pointer-events: none; opacity: 0.8; z-index: 5; }
  .ghost { position: fixed; z-index: 100; pointer-events: none; padding: var(--sp-2) var(--sp-4); min-width: 9rem; border: 1px solid var(--accent); border-radius: var(--radius); background: color-mix(in srgb, var(--accent) 22%, var(--surface-1)); opacity: 0.7; font-size: var(--fs-sm); font-weight: 600; box-shadow: var(--shadow); }
  @media (max-width: 700px) {
    .area { flex-direction: column; }
    .dock { flex-direction: row; width: auto; margin: 0 2px 2px; padding: 0 var(--sp-2); overflow-x: auto; min-height: 2.9rem; }
    .ws { flex-direction: column; overflow: auto; }
    .stack { flex: none !important; }
    .block { flex: none !important; min-height: 14rem; }
    .split.v { display: none; }
  }
</style>
