// Modèle de disposition : pur (sans Svelte, sans Tauri), donc testable avec `node --test`.
// Une disposition = des piles (colonnes) de blocs. Les tailles sont des poids dont la somme vaut 1.

import { parseSpec, type IconSpec } from '../icons/model.ts'

export type Block = {
  id: string
  type: string // clé du registre (registry.ts) : ajouter un type n'exige aucun changement ici
  props: Record<string, unknown>
  size: number
  collapsed?: boolean
}
export type Stack = { id: string; size: number; blocks: Block[] }
// `dock` : blocs mis de côté (barre de droite), hors des piles ; leurs terminaux continuent de tourner.
export type Layout = { version: 1; stacks: Stack[]; dock?: Block[] }

export const MAX_STACKS = 4
export const MIN_SIZE = 0.1

let counter = 0
export const uid = (p: string) => `${p}${Date.now().toString(36)}${(counter++).toString(36)}`

export function equalize<T extends { size: number }>(items: T[]): T[] {
  return items.map((i) => ({ ...i, size: 1 / items.length }))
}

export function makeBlock(type: string, props: Record<string, unknown> = {}): Block {
  return { id: uid('b'), type, props, size: 1 }
}

export function normalize(layout: Layout): Layout {
  const stacks = layout.stacks
    .map((s) => ({ ...s, blocks: equalizeIfInvalid(s.blocks) }))
    .filter((s) => s.blocks.length > 0)
  return { version: 1, stacks: equalizeIfInvalid(stacks), ...(layout.dock?.length ? { dock: layout.dock } : {}) }
}

function equalizeIfInvalid<T extends { size: number }>(items: T[]): T[] {
  if (items.length === 0) return items
  const sum = items.reduce((a, i) => a + i.size, 0)
  if (!(sum > 0) || items.some((i) => !(i.size > 0))) return equalize(items)
  return items.map((i) => ({ ...i, size: i.size / sum }))
}

export function defaultLayout(): Layout {
  return normalize({
    version: 1,
    stacks: [
      { id: uid('s'), size: 1, blocks: [makeBlock('state'), makeBlock('log')] },
      { id: uid('s'), size: 1, blocks: [makeBlock('terminal', { kind: 'wsl' }), makeBlock('config')] },
    ],
  })
}

// Déplace le séparateur après l'élément `i` de `deltaFraction` (fraction du conteneur).
export function resizeAt<T extends { size: number }>(items: T[], i: number, deltaFraction: number): T[] {
  if (i < 0 || i >= items.length - 1) return items
  const a = items[i].size
  const b = items[i + 1].size
  const total = a + b
  const na = Math.min(Math.max(a + deltaFraction, MIN_SIZE), total - MIN_SIZE)
  const out = items.slice()
  out[i] = { ...items[i], size: na }
  out[i + 1] = { ...items[i + 1], size: total - na }
  return out
}

export type DropTarget =
  | { kind: 'stack'; stackId: string; index: number }
  | { kind: 'new-stack'; at: number } // position de la nouvelle pile (0 = tout à gauche)

export function findBlock(layout: Layout, blockId: string): { stack: number; index: number } | null {
  for (let s = 0; s < layout.stacks.length; s++) {
    const index = layout.stacks[s].blocks.findIndex((b) => b.id === blockId)
    if (index >= 0) return { stack: s, index }
  }
  return null
}

export function moveBlock(layout: Layout, blockId: string, target: DropTarget): Layout {
  const from = findBlock(layout, blockId)
  if (!from) return layout
  const block = layout.stacks[from.stack].blocks[from.index]
  if (target.kind === 'new-stack' && layout.stacks.length >= MAX_STACKS) return layout
  // retrait
  let stacks = layout.stacks.map((s) => ({ ...s, blocks: s.blocks.filter((b) => b.id !== blockId) }))
  // ajout
  if (target.kind === 'stack') {
    const t = stacks.findIndex((s) => s.id === target.stackId)
    if (t < 0) return layout
    let index = target.index
    if (layout.stacks[from.stack].id === target.stackId && from.index < index) index -= 1
    const blocks = stacks[t].blocks.slice()
    blocks.splice(Math.min(Math.max(index, 0), blocks.length), 0, { ...block, size: 1 })
    stacks[t] = { ...stacks[t], blocks: equalize(blocks) }
  } else {
    const fresh: Stack = { id: uid('s'), size: 1 / (stacks.length + 1), blocks: [{ ...block, size: 1 }] }
    const at = Math.min(Math.max(target.at, 0), stacks.length)
    stacks = stacks.slice()
    stacks.splice(at, 0, fresh)
  }
  return normalize({ ...layout, stacks })
}

export function addBlock(layout: Layout, block: Block, stackIndex = layout.stacks.length - 1): Layout {
  if (layout.stacks.length === 0) return normalize({ ...layout, stacks: [{ id: uid('s'), size: 1, blocks: [block] }] })
  const i = Math.min(Math.max(stackIndex, 0), layout.stacks.length - 1)
  const stacks = layout.stacks.map((s, k) => (k === i ? { ...s, blocks: equalize([...s.blocks, block]) } : s))
  return normalize({ ...layout, stacks })
}

export function removeBlock(layout: Layout, blockId: string): Layout {
  const stacks = layout.stacks.map((s) => ({ ...s, blocks: equalize(s.blocks.filter((b) => b.id !== blockId)) }))
  const dock = (layout.dock ?? []).filter((b) => b.id !== blockId)
  return normalize({ version: 1, stacks, dock })
}

// Met un bloc de côté (barre de droite) ; le remet dans la dernière pile.
export function dockBlock(layout: Layout, blockId: string): Layout {
  const at = findBlock(layout, blockId)
  if (!at) return layout
  const block = { ...layout.stacks[at.stack].blocks[at.index], size: 1 }
  const rest = removeBlock(layout, blockId)
  return normalize({ ...rest, dock: [...(rest.dock ?? []), block] })
}

export function undockBlock(layout: Layout, blockId: string): Layout {
  const block = (layout.dock ?? []).find((b) => b.id === blockId)
  if (!block) return layout
  const without = normalize({ ...layout, stacks: layout.stacks, dock: (layout.dock ?? []).filter((b) => b.id !== blockId) })
  return addBlock(without, block)
}

export function setBlockProps(layout: Layout, blockId: string, props: Record<string, unknown>): Layout {
  return {
    ...layout,
    stacks: layout.stacks.map((s) => ({
      ...s,
      blocks: s.blocks.map((b) => (b.id === blockId ? { ...b, props: { ...b.props, ...props } } : b)),
    })),
    ...(layout.dock ? { dock: layout.dock.map((b) => (b.id === blockId ? { ...b, props: { ...b.props, ...props } } : b)) } : {}),
  }
}

// Lecture tolérante : null si la disposition est invalide ou inconnue.
function tryParseLayout(raw: unknown): Layout | null {
  try {
    const l = raw as Layout
    if (l?.version !== 1 || !Array.isArray(l.stacks)) return null
    const valid = (b: Block) => b && typeof b.id === 'string' && typeof b.type === 'string'
    const ok = l.stacks.every((s) => typeof s.id === 'string' && Array.isArray(s.blocks) && s.blocks.every(valid))
    if (!ok) return null
    const fix = (b: Block): Block => ({ ...b, props: b.props ?? {}, size: b.size > 0 ? b.size : 1 })
    const dock = Array.isArray(l.dock) ? l.dock.filter(valid).map(fix) : []
    const n = normalize({ version: 1, stacks: l.stacks.slice(0, MAX_STACKS).map((s) => ({ ...s, blocks: s.blocks.map(fix) })), dock })
    return n.stacks.length || dock.length ? n : null
  } catch {
    return null
  }
}

// Une disposition invalide retombe sur le défaut (jamais de plantage).
export const parseLayout = (raw: unknown): Layout => tryParseLayout(raw) ?? defaultLayout()

// Un affichage peut être vide (pas de disposition par défaut : ses blocs doivent nommer une distro).
export const emptyLayout = (): Layout => ({ version: 1, stacks: [] })
export function parseViewLayout(raw: unknown): Layout {
  const l = raw as Layout | null
  if (l?.version === 1 && Array.isArray(l.stacks) && l.stacks.length === 0 && !l.dock?.length) return emptyLayout()
  return tryParseLayout(raw) ?? emptyLayout()
}

// --- Document complet (layouts.json) : une disposition courante par distro + des dispositions nommées.
export type LayoutsDoc = {
  version: 1
  distros: Record<string, Layout>
  presets: Record<string, Layout> // dispositions nommées, applicables à toute distro
  views: View[] // affichages libres ; leur disposition est dans `distros` sous la clé viewKey(id)
}

// Affichage : espace de travail non lié à une distro, dont les blocs portent leur propre distro (props.distro).
export type ProjectRoot = { distro: string; path: string } // distro vide = dossier Windows
export type View = { id: string; name: string; icon?: IconSpec; root?: ProjectRoot }

export function parseRoot(raw: unknown): ProjectRoot | undefined {
  const r = raw as Partial<ProjectRoot> | null
  return r && typeof r.distro === 'string' && typeof r.path === 'string' && r.path ? { distro: r.distro, path: r.path } : undefined
}
export const viewKey = (id: string) => `view:${id}`
export const isViewKey = (k: string) => k.startsWith('view:')

export const emptyDoc = (): LayoutsDoc => ({ version: 1, distros: {}, presets: {}, views: [] })

export function parseDoc(raw: unknown): LayoutsDoc {
  const doc = emptyDoc()
  try {
    const r = raw as Partial<LayoutsDoc> | null
    if (!r || r.version !== 1) return doc
    for (const [k, v] of Object.entries(r.distros ?? {})) doc.distros[k] = isViewKey(k) ? parseViewLayout(v) : parseLayout(v)
    for (const [k, v] of Object.entries(r.presets ?? {})) doc.presets[k] = parseLayout(v)
    const seen = new Set<string>()
    for (const v of Array.isArray(r.views) ? r.views : []) {
      const o = v as Partial<View> | null
      if (!o || typeof o.id !== 'string' || !o.id || typeof o.name !== 'string' || seen.has(o.id)) continue
      seen.add(o.id)
      const icon = parseSpec(o.icon)
      const root = parseRoot(o.root)
      doc.views.push({ id: o.id, name: o.name, ...(icon ? { icon } : {}), ...(root ? { root } : {}) })
    }
  } catch {
    /* document illisible : repart de zéro */
  }
  return doc
}

// Copie d'une disposition avec de nouveaux identifiants (un preset appliqué ne partage aucun id).
export function cloneLayout(l: Layout): Layout {
  return {
    version: 1,
    stacks: l.stacks.map((s) => ({
      ...s,
      id: uid('s'),
      blocks: s.blocks.map((b) => ({ ...b, id: uid('b'), props: structuredClone(b.props) })),
    })),
    ...(l.dock?.length ? { dock: l.dock.map((b) => ({ ...b, id: uid('b'), props: structuredClone(b.props) })) } : {}),
  }
}

export function savePreset(doc: LayoutsDoc, name: string, layout: Layout): LayoutsDoc {
  const n = name.trim()
  if (!n) return doc
  return { ...doc, presets: { ...doc.presets, [n]: cloneLayout(layout) } }
}

export function deletePreset(doc: LayoutsDoc, name: string): LayoutsDoc {
  const presets = { ...doc.presets }
  delete presets[name]
  return { ...doc, presets }
}

export function applyPreset(doc: LayoutsDoc, name: string): Layout | null {
  const p = doc.presets[name]
  return p ? cloneLayout(p) : null
}

// --- Affichages : opérations pures sur le document.
export function addView(doc: LayoutsDoc, name: string): { doc: LayoutsDoc; id: string } {
  const id = uid('v')
  const n = name.trim() || 'Affichage'
  const layout: Layout = { version: 1, stacks: [] }
  return { id, doc: { ...doc, views: [...doc.views, { id, name: n }], distros: { ...doc.distros, [viewKey(id)]: layout } } }
}

export function updateView(doc: LayoutsDoc, id: string, patch: Partial<Omit<View, 'id'>>): LayoutsDoc {
  return { ...doc, views: doc.views.map((v) => (v.id === id ? { ...v, ...patch, name: (patch.name ?? v.name).trim() || v.name } : v)) }
}

export function removeView(doc: LayoutsDoc, id: string): LayoutsDoc {
  const distros = { ...doc.distros }
  delete distros[viewKey(id)]
  return { ...doc, views: doc.views.filter((v) => v.id !== id), distros }
}

export function moveView(doc: LayoutsDoc, id: string, delta: -1 | 1): LayoutsDoc {
  const i = doc.views.findIndex((v) => v.id === id)
  const j = i + delta
  if (i < 0 || j < 0 || j >= doc.views.length) return doc
  const views = [...doc.views]
  ;[views[i], views[j]] = [views[j], views[i]]
  return { ...doc, views }
}

// Copie d'un affichage (disposition clonée, nouveaux identifiants).
export function duplicateView(doc: LayoutsDoc, id: string): { doc: LayoutsDoc; id: string } | null {
  const v = doc.views.find((x) => x.id === id)
  if (!v) return null
  const nid = uid('v')
  const copy: View = { ...v, id: nid, name: `${v.name} (copie)` }
  const layout = cloneLayout(doc.distros[viewKey(id)] ?? { version: 1, stacks: [] })
  return { id: nid, doc: { ...doc, views: [...doc.views, copy], distros: { ...doc.distros, [viewKey(nid)]: layout } } }
}

// Distro d'un bloc : la sienne (affichage) ou celle de l'espace qui le contient.
export const blockDistro = (b: Block, fallback: string): string =>
  typeof b.props.distro === 'string' && b.props.distro ? b.props.distro : fallback
