<script lang="ts">
  import { untrack } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import {
    defaultLayout, emptyLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, setBlockProps,
    findBlock, dockBlock, undockBlock, MAX_STACKS, blockDistro, type ProjectRoot, type Layout, type DropTarget, type Block,
  } from './model'
  import { registry, def } from './registry'
  import { app, display } from '../state/app.svelte'
  import Icon from '../icons/Icon.svelte'
  import { launchers } from '../state/launchers.svelte'
  import { availableIn, launchProps } from '../launchers/model'
  import { layouts, layoutFor, setLayout, saveAsPreset, removePreset, presetLayout } from '../state/layouts.svelte'
  import { promptText, confirmAction } from '../ui/feedback.svelte'
  import Menu from '../ui/Menu.svelte'
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
        for (const t of prevTerms) if (!ids.has(t) && !keep.delete(t)) void invoke('pty_close', { id: t }).catch(() => {})
      }
      for (const t of ids) keep.delete(t) // rattaché : il redevient un bloc ordinaire
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
  let drag = $state<{ id: string; x: number; y: number; target: DropTarget | null; ind: { l: number; t: number; w: number; h: number } | null } | null>(null)

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
      drag = { id, x: ev.clientX, y: ev.clientY, target, ind }
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
    return props.kind === 'wsl' ? 'Terminal WSL' : props.start === 'profile' ? 'PowerShell (profil)' : 'PowerShell (disque distro)'
  }
  // Dans un affichage, le titre rappelle la distro du bloc.
  const titleOf = (type: string, props: Record<string, unknown>) => {
    const t = baseTitle(type, props)
    return inView && typeof props.distro === 'string' && props.distro ? `${t} · ${display(props.distro)}` : t
  }

  // Lanceurs : un terminal qui exécute la commande, dans le dossier du projet de l'affichage s'il y en a un.
  const launcherItems = $derived<MenuItem[]>(
    launchers.doc.launchers
      .filter((l) => availableIn(l, inView ? null : distro))
      .flatMap((l) => {
        const targets = l.target === 'windows' ? [''] : l.distro ? [l.distro] : inView ? app.distros.map((x) => x.name) : [distro]
        return targets.map((t) => ({
          label: `▶ ${l.name}${l.target === 'wsl' && !l.distro && inView ? ' · ' + display(t) : ''}`,
          onselect: () => (layout = addBlock(layout, makeBlock('terminal', launchProps(l, t, projectRoot)))),
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
    { label: 'Enregistrer cette disposition…', onselect: () => void savePresetAsk() },
    ...Object.keys(layouts.doc.presets).flatMap((n) => [
      { label: `Appliquer « ${n} »`, onselect: () => { const l = presetLayout(n); if (l) layout = l } },
      { label: `Supprimer « ${n} »…`, danger: true, onselect: () => void deletePresetAsk(n) },
    ]),
    { label: inView ? 'Vider l’affichage' : 'Revenir à la disposition par défaut', onselect: () => (layout = inView ? emptyLayout() : defaultLayout()) },
  ])

  async function savePresetAsk() {
    const n = await promptText({ title: 'Nom de la disposition', message: 'Elle pourra être appliquée à n’importe quelle distribution.' })
    if (n?.trim()) saveAsPreset(n, $state.snapshot(layout))
  }
  async function deletePresetAsk(n: string) {
    if (await confirmAction({ title: `Supprimer la disposition « ${n} » ?`, confirmLabel: 'Supprimer', danger: true })) removePreset(n)
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
    { label: '◀ Pile précédente', onselect: () => shift(b.id, 'left') },
    { label: '▶ Pile suivante', onselect: () => shift(b.id, 'right') },
    { label: '▲ Monter', onselect: () => shift(b.id, 'up') },
    { label: '▼ Descendre', onselect: () => shift(b.id, 'down') },
    { label: '⇥ Mettre de côté', onselect: () => (layout = dockBlock(layout, b.id)) },
    { label: '⧉ Détacher dans une fenêtre', onselect: () => detach(b) },
    { label: '✕ Fermer', danger: true, onselect: () => (layout = removeBlock(layout, b.id)) },
  ]
</script>

<div class="bar">
  <Menu items={addItems} label="Ajouter un bloc">{#snippet trigger()}+ Bloc{/snippet}</Menu>
  <Menu items={layoutItems} label="Dispositions">{#snippet trigger()}Dispositions{/snippet}</Menu>
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
            <Menu items={blockMenu(b)} label="Menu du bloc" />
          </header>
          <div class="body">
            {#if d}
              {@const C = d.component}
              {#if needsDistro(b.type, b.props) && app.loaded && !app.distros.some((x) => x.name === blockDistro(b, distro))}
                <p class="pad">Distribution « {blockDistro(b, distro) || '?'} » absente.</p>
              {:else}
              <C distro={blockDistro(b, distro)} root={projectRoot} open={openBlock} block={b} update={(p) => (layout = setBlockProps(layout, b.id, p))} />
              {/if}
            {:else}
              <p class="pad">Bloc inconnu « {b.type} ».</p>
            {/if}
          </div>
        </article>
      {/each}
    </section>
  {/each}
  {#if layout.stacks.length === 0 && !layout.dock?.length}
    <p class="empty">Affichage vide. « + Bloc » ajoute un terminal, un journal… de n’importe quelle distribution.</p>
  {/if}
  {#if drag?.ind}
    <div class="ind" style:left="{drag.ind.l}px" style:top="{drag.ind.t}px" style:width="{drag.ind.w}px" style:height="{drag.ind.h}px"></div>
  {/if}
</div>
{#if layout.dock?.length}
  <aside class="dock" aria-label="Blocs mis de côté">
    {#each layout.dock as b (b.id)}
      <button class="dk" title="{titleOf(b.type, b.props)} : remettre" aria-label="Remettre {titleOf(b.type, b.props)}" onclick={() => (layout = undockBlock(layout, b.id))}>
        <Icon spec={{ kind: 'symbol', value: def(b.type)?.icon ?? 'box' }} size={18} />
      </button>
    {/each}
  </aside>
{/if}
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
  .dock { flex: none; display: flex; flex-direction: column; gap: var(--sp-1); padding: var(--sp-2) var(--sp-1); border-left: 1px solid var(--border); background: var(--surface-1); overflow-y: auto; }
  .dk { display: grid; place-items: center; width: 2.2rem; height: 2.2rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface-2); color: var(--text); cursor: pointer; }
  .dk:hover, .dk:focus-visible { border-color: var(--accent); outline: none; }
  .ws { position: relative; display: flex; flex: 1; min-height: 0; min-width: 0; padding: 2px; }
  .stack { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .block { display: flex; flex-direction: column; min-height: 0; border: 1px solid var(--border); border-radius: var(--radius); margin: 2px; background: var(--surface-1); }
  .block.dragging { opacity: 0.35; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--sp-2) 0 var(--sp-3); background: var(--surface-2); border-radius: var(--radius) var(--radius) 0 0; cursor: grab; user-select: none; touch-action: none; }
  .t { font-size: var(--fs-sm); font-weight: 600; }
  .body { flex: 1; min-height: 0; overflow: auto; container-type: inline-size; border-radius: 0 0 var(--radius) var(--radius); }
  .split { flex: none; touch-action: none; }
  .split.v { width: 6px; cursor: col-resize; }
  .split.h { height: 6px; cursor: row-resize; }
  .split:hover { background: var(--surface-3); }
  .ind { position: absolute; background: var(--accent); border-radius: 2px; pointer-events: none; opacity: 0.8; z-index: 5; }
  .ghost { position: fixed; z-index: 100; pointer-events: none; padding: var(--sp-2) var(--sp-4); min-width: 9rem; border: 1px solid var(--accent); border-radius: var(--radius); background: color-mix(in srgb, var(--accent) 22%, var(--surface-1)); opacity: 0.7; font-size: var(--fs-sm); font-weight: 600; box-shadow: var(--shadow); }
  @media (max-width: 700px) {
    .area { flex-direction: column; }
    .dock { flex-direction: row; border-left: 0; border-top: 1px solid var(--border); overflow-x: auto; }
    .ws { flex-direction: column; overflow: auto; }
    .stack { flex: none !important; }
    .block { flex: none !important; min-height: 14rem; }
    .split.v { display: none; }
  }
</style>
