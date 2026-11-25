// Lanceurs : launchers.json (Rust), chargés au démarrage, sauvegardés à chaque modification.
import { invoke } from '@tauri-apps/api/core'
import { emptyLaunchers, parseLaunchers, upsertLauncher, removeLauncher, type Launcher } from '../launchers/model'

export const launchers = $state({ doc: emptyLaunchers(), loaded: false })

export async function loadLaunchers() {
  try {
    launchers.doc = parseLaunchers(await invoke('store_load', { file: 'launchers.json' }))
  } catch {
    launchers.doc = emptyLaunchers()
  }
  launchers.loaded = true
}

async function save() {
  try {
    await invoke('store_save', { file: 'launchers.json', doc: $state.snapshot(launchers.doc) })
  } catch {
    /* reste en mémoire pour la session */
  }
}

export async function saveLauncher(l: Launcher) {
  launchers.doc = upsertLauncher(launchers.doc, l)
  await save()
}

export async function deleteLauncher(id: string) {
  launchers.doc = removeLauncher(launchers.doc, id)
  await save()
}
