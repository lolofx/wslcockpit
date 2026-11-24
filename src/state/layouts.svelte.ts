// Dispositions : chargées une fois depuis layouts.json (Rust), sauvegardées avec un délai.
import { invoke } from '@tauri-apps/api/core'
import { emptyDoc, parseDoc, defaultLayout, savePreset, deletePreset, applyPreset, type Layout, type LayoutsDoc } from '../workspace/model'

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

export const layoutFor = (distro: string): Layout => layouts.doc.distros[distro] ?? defaultLayout()

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
