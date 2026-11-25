// Commandes de la palette (Ctrl+K) : une seule définition, pour que tout soit atteignable au clavier.
import { app, act, openTerminal, display, makeDefault, shutdownAll, rename } from '../state/app.svelte'

import { chooseDistroIcon } from '../state/icons.svelte'
import type { Command } from './filter'
import { layouts } from '../state/layouts.svelte'
import { newView } from './menus'
import { viewKey } from '../workspace/model'
export type Nav = { go: (view: string) => void; current: () => string | null }

export function buildCommands(nav: Nav): Command[] {
  const out: Command[] = [
    { id: 'overview', label: 'Vue d’ensemble', run: () => nav.go('overview') },
    { id: 'settings', label: 'Configuration (wsl.conf, .wslconfig)', run: () => nav.go('settings') },
    { id: 'newview', label: 'Nouvel affichage…', run: () => void newView(nav.go) },
    { id: 'create', label: 'Nouvelle distribution…', run: () => (app.ops = { mode: 'create', source: '' }) },
    { id: 'wslupdate', label: 'Mettre à jour WSL…', run: () => (app.ops = { mode: 'wslupdate', source: '' }) },
    { id: 'import', label: 'Importer une distribution…', run: () => (app.ops = { mode: 'import', source: '' }) },
    { id: 'shutdown', label: 'Tout arrêter', run: () => void shutdownAll() },
  ]
  for (const v of layouts.doc.views) out.push({ id: `view:${v.id}`, label: `Affichage ${v.name}`, run: () => nav.go(viewKey(v.id)) })
  for (const d of app.distros) {
    const n = display(d.name)
    out.push({ id: `go:${d.name}`, label: `Aller à ${n}`, run: () => nav.go(d.name) })
    const run = d.state === 'running'
    out.push(
      run
        ? { id: `stop:${d.name}`, label: `Arrêter ${n}`, run: () => void act('stop', d.name) }
        : { id: `start:${d.name}`, label: `Démarrer ${n}`, run: () => void act('start', d.name) },
      { id: `term:${d.name}`, label: `Terminal ${n}`, run: () => openTerminal(d.name) },
      { id: `termext:${d.name}`, label: `Terminal externe ${n}`, run: () => void act('terminal', d.name) },
      { id: `icon:${d.name}`, label: `Icône de ${n}…`, run: () => void chooseDistroIcon(d.name, n) },
      { id: `rename:${d.name}`, label: `Alias de ${n}…`, run: () => void rename(d.name) },
      { id: `export:${d.name}`, label: `Exporter ${n}…`, run: () => (app.ops = { mode: 'export', source: d.name }) },
      { id: `clone:${d.name}`, label: `Cloner ${n}…`, run: () => (app.ops = { mode: 'clone', source: d.name }) },
      { id: `delete:${d.name}`, label: `Supprimer ${n}…`, run: () => (app.deleting = d.name) },
    )
    if (run) out.push({ id: `restart:${d.name}`, label: `Redémarrer ${n}`, run: () => void act('restart', d.name) })
    if (!d.is_default) out.push({ id: `default:${d.name}`, label: `${n} : définir par défaut`, run: () => void makeDefault(d.name) })
  }
  return out
}
