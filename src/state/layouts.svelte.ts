// Dispositions : chargées une fois depuis layouts.json (Rust), sauvegardées avec un délai.
import { invoke } from '@tauri-apps/api/core'
import { emptyLayout, isViewKey, emptyDoc, parseDoc, defaultLayout, savePreset, deletePreset, applyPreset, addView, updateView, removeView, moveView, duplicateView, type Layout, type LayoutsDoc, type View } from '../workspace/model'

// `rev` change quand la disposition est modifiée hors de l'espace de travail ouvert (bloc rattaché).
export const layouts = $state({ doc: emptyDoc() as LayoutsDoc, loaded: false, rev: 0 })

let timer: ReturnType<typeof setTimeout> | undefined

function schedule() {
  clearTimeout(timer)
  timer = setTimeout(flush, 400)
}

export async function flush() {
  clearTimeout(timer)
  try {
    await invoke('layouts_save', { doc: $state.snapshot(layouts.doc) })
  } catch {
    /* un échec d'écriture ne doit pas gêner l'usage ; la disposition reste en mémoire */
  }
}

export async function loadLayouts() {
  try {
    layouts.doc = parseDoc(await invoke('layouts_load'))
  } catch {
    layouts.doc = emptyDoc()
  }
  layouts.loaded = true
}

export const layoutFor = (key: string): Layout => layouts.doc.distros[key] ?? (isViewKey(key) ? emptyLayout() : defaultLayout())

export function setLayout(distro: string, l: Layout) {
  layouts.doc = { ...layouts.doc, distros: { ...layouts.doc.distros, [distro]: l } }
  schedule()
}

export function setLayoutExternal(distro: string, l: Layout) {
  setLayout(distro, l)
  layouts.rev += 1
}

export function saveAsPreset(name: string, l: Layout) {
  layouts.doc = savePreset(layouts.doc, name, l)
  schedule()
}

export function removePreset(name: string) {
  layouts.doc = deletePreset(layouts.doc, name)
  schedule()
}

export const presetLayout = (name: string) => applyPreset(layouts.doc, name)

// --- Affichages (espaces libres multi-distros)
export const views = () => layouts.doc.views
export const findView = (id: string): View | undefined => layouts.doc.views.find((v) => v.id === id)

export function createView(name: string): string {
  const r = addView(layouts.doc, name)
  layouts.doc = r.doc
  schedule()
  return r.id
}
export function patchView(id: string, patch: Partial<Omit<View, 'id'>>) {
  layouts.doc = updateView(layouts.doc, id, patch)
  schedule()
}
export function deleteView(id: string) {
  layouts.doc = removeView(layouts.doc, id)
  schedule()
}
export function shiftView(id: string, delta: -1 | 1) {
  layouts.doc = moveView(layouts.doc, id, delta)
  schedule()
}
export function copyView(id: string): string | null {
  const r = duplicateView(layouts.doc, id)
  if (!r) return null
  layouts.doc = r.doc
  schedule()
  return r.id
}
