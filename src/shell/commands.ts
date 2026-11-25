import { t } from '../i18n/index.svelte'
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
    { id: 'overview', label: t('commands.overview'), run: () => nav.go('overview') },
    { id: 'settings', label: t('commands.settings'), run: () => nav.go('settings') },
    { id: 'newview', label: t('commands.newView'), run: () => void newView(nav.go) },
    { id: 'create', label: t('commands.create'), run: () => (app.ops = { mode: 'create', source: '' }) },
    { id: 'wslupdate', label: t('commands.wslUpdate'), run: () => (app.ops = { mode: 'wslupdate', source: '' }) },
    { id: 'import', label: t('commands.import'), run: () => (app.ops = { mode: 'import', source: '' }) },
    { id: 'shutdown', label: t('commands.shutdownAll'), run: () => void shutdownAll() },
  ]
  for (const v of layouts.doc.views) out.push({ id: `view:${v.id}`, label: t('commands.view', { name: v.name }), run: () => nav.go(viewKey(v.id)) })
  for (const d of app.distros) {
    const n = display(d.name)
    out.push({ id: `go:${d.name}`, label: t('commands.goto', { name: n }), run: () => nav.go(d.name) })
    const run = d.state === 'running'
    out.push(
      run
        ? { id: `stop:${d.name}`, label: t('commands.stop', { name: n }), run: () => void act('stop', d.name) }
        : { id: `start:${d.name}`, label: t('commands.start', { name: n }), run: () => void act('start', d.name) },
      { id: `term:${d.name}`, label: t('commands.terminal', { name: n }), run: () => openTerminal(d.name) },
      { id: `termext:${d.name}`, label: t('commands.terminalExternal', { name: n }), run: () => void act('terminal', d.name) },
      { id: `icon:${d.name}`, label: t('commands.icon', { name: n }), run: () => void chooseDistroIcon(d.name, n) },
      { id: `rename:${d.name}`, label: t('commands.alias', { name: n }), run: () => void rename(d.name) },
      { id: `export:${d.name}`, label: t('commands.export', { name: n }), run: () => (app.ops = { mode: 'export', source: d.name }) },
      { id: `clone:${d.name}`, label: t('commands.clone', { name: n }), run: () => (app.ops = { mode: 'clone', source: d.name }) },
      { id: `delete:${d.name}`, label: t('commands.delete', { name: n }), run: () => (app.deleting = d.name) },
    )
    if (run) out.push({ id: `restart:${d.name}`, label: t('commands.restart', { name: n }), run: () => void act('restart', d.name) })
    if (!d.is_default) out.push({ id: `default:${d.name}`, label: t('commands.makeDefault', { name: n }), run: () => void makeDefault(d.name) })
  }
  return out
}
