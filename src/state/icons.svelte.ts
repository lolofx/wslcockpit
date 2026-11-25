import { t } from '../i18n/index.svelte'
// Icônes des distributions : icons.json (Rust), chargées une fois, sauvegardées à chaque choix.
import { invoke } from '@tauri-apps/api/core'
import { emptyIcons, guessLogo, parseIcons, setDistroIcon, type IconSpec } from '../icons/model'
import { pickIcon } from '../icons/picker.svelte'

export const icons = $state({ doc: emptyIcons(), loaded: false })

export async function loadIcons() {
  try {
    icons.doc = parseIcons(await invoke('store_load', { file: 'icons.json' }))
  } catch {
    icons.doc = emptyIcons()
  }
  icons.loaded = true
}

export const distroIcon = (name: string): IconSpec | null => icons.doc.distros[name] ?? null

export async function chooseDistroIcon(name: string, label: string) {
  const r = await pickIcon({ title: t('app.icon.title', { label }), label, current: distroIcon(name), suggestion: guessLogo(name) })
  if (r === undefined) return
  icons.doc = setDistroIcon(icons.doc, name, r)
  try {
    await invoke('store_save', { file: 'icons.json', doc: $state.snapshot(icons.doc) })
  } catch {
    /* l'icône reste en mémoire pour la session */
  }
}
