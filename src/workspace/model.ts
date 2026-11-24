// Modèle de disposition : pur (sans Svelte, sans Tauri), donc testable avec `node --test`.
// Une disposition = des piles (colonnes) de blocs. Les tailles sont des poids dont la somme vaut 1.

export type Block = {
  id: string
  type: string // clé du registre (registry.ts) : ajouter un type n'exige aucun changement ici
  props: Record<string, unknown>
  size: number
  collapsed?: boolean
}
export type Stack = { id: string; size: number; blocks: Block[] }
export type Layout = { version: 1; stacks: Stack[] }

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
  return { version: 1, stacks: equalizeIfInvalid(stacks) }
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
  return normalize({ version: 1, stacks })
}

export function addBlock(layout: Layout, block: Block, stackIndex = layout.stacks.length - 1): Layout {
  if (layout.stacks.length === 0) return normalize({ version: 1, stacks: [{ id: uid('s'), size: 1, blocks: [block] }] })
  const i = Math.min(Math.max(stackIndex, 0), layout.stacks.length - 1)
  const stacks = layout.stacks.map((s, k) => (k === i ? { ...s, blocks: equalize([...s.blocks, block]) } : s))
  return normalize({ version: 1, stacks })
}

export function removeBlock(layout: Layout, blockId: string): Layout {
  const stacks = layout.stacks.map((s) => ({ ...s, blocks: equalize(s.blocks.filter((b) => b.id !== blockId)) }))
  return normalize({ version: 1, stacks })
}

export function setBlockProps(layout: Layout, blockId: string, props: Record<string, unknown>): Layout {
  return {
    ...layout,
    stacks: layout.stacks.map((s) => ({
      ...s,
      blocks: s.blocks.map((b) => (b.id === blockId ? { ...b, props: { ...b.props, ...props } } : b)),
    })),
  }
}

// Lecture tolérante : toute disposition invalide ou inconnue retombe sur le défaut (jamais de plantage).
export function parseLayout(raw: unknown): Layout {
  try {
    const l = raw as Layout
    if (l?.version !== 1 || !Array.isArray(l.stacks)) return defaultLayout()
    const ok = l.stacks.every(
      (s) => typeof s.id === 'string' && Array.isArray(s.blocks) && s.blocks.every((b) => typeof b.id === 'string' && typeof b.type === 'string'),
    )
    if (!ok) return defaultLayout()
    const n = normalize({ ...l, stacks: l.stacks.slice(0, MAX_STACKS).map((s) => ({ ...s, blocks: s.blocks.map((b) => ({ ...b, props: b.props ?? {} })) })) })
    return n.stacks.length ? n : defaultLayout()
  } catch {
    return defaultLayout()
  }
}
