import { chooseDistroIcon } from '../state/icons.svelte'
import { app, act, openTerminal, display, makeDefault, rename, type Distro } from '../state/app.svelte'

export type MenuItem = { label: string; onselect: () => void; danger?: boolean; disabled?: boolean }

// Menu d'une distro : identique sur la carte, dans la barre haute et le rail (une seule définition).
export function distroMenu(d: Distro): MenuItem[] {
  const running = d.state === 'running'
  const busy = app.busy !== null
  const items: MenuItem[] = [
    { label: 'Terminal', onselect: () => openTerminal(d.name) },
    { label: 'Terminal externe', onselect: () => void act('terminal', d.name), disabled: busy },
    ...(running ? [{ label: 'Redémarrer', onselect: () => void act('restart', d.name), disabled: busy }] : []),
    { label: 'Changer l’icône…', onselect: () => void chooseDistroIcon(d.name, display(d.name)) },
    { label: 'Modifier l’alias…', onselect: () => void rename(d.name) },
    ...(d.is_default ? [] : [{ label: 'Définir par défaut', onselect: () => void makeDefault(d.name), disabled: busy }]),
    { label: 'Exporter…', onselect: () => (app.ops = { mode: 'export', source: d.name }), disabled: busy },
    { label: 'Cloner…', onselect: () => (app.ops = { mode: 'clone', source: d.name }), disabled: busy },
    ...(running
      ? []
      : [
          { label: `Convertir en WSL${d.version === 2 ? 1 : 2}…`, onselect: () => (app.ops = { mode: 'convert', source: d.name, version: d.version }), disabled: busy },
          { label: 'Compacter (sparse)…', onselect: () => (app.ops = { mode: 'sparse', source: d.name }), disabled: busy },
        ]),
    { label: 'Supprimer…', onselect: () => (app.deleting = d.name), danger: true, disabled: busy || d.state === 'installing' },
  ]
  return items
}

export const title = (d: Distro) => display(d.name)

// --- Affichages
import { layouts, layoutFor, createView, patchView, deleteView, shiftView, copyView, findView } from '../state/layouts.svelte'
import { viewKey } from '../workspace/model'
import { promptText, confirmAction } from '../ui/feedback.svelte'
import { pickIcon } from '../icons/picker.svelte'
import { invoke } from '@tauri-apps/api/core'

export async function newView(go: (v: string) => void) {
  const n = await promptText({ title: 'Nouvel affichage', message: 'Un espace libre où mettre les blocs de plusieurs distributions.', value: '' })
  if (n === null) return
  go(viewKey(createView(n)))
}

export function viewMenu(id: string, go: (v: string) => void): MenuItem[] {
  const v = findView(id)
  if (!v) return []
  const i = layouts.doc.views.findIndex((x) => x.id === id)
  return [
    {
      label: 'Renommer…',
      onselect: async () => {
        const n = await promptText({ title: 'Nom de l’affichage', value: v.name })
        if (n?.trim()) patchView(id, { name: n })
      },
    },
    {
      label: 'Changer l’icône…',
      onselect: async () => {
        const r = await pickIcon({ title: `Icône de « ${v.name} »`, label: v.name, current: v.icon ?? null })
        if (r !== undefined) patchView(id, { icon: r ?? undefined })
      },
    },
    { label: 'Dupliquer', onselect: () => { const c = copyView(id); if (c) go(viewKey(c)) } },
    { label: '▲ Monter', onselect: () => shiftView(id, -1), disabled: i <= 0 },
    { label: '▼ Descendre', onselect: () => shiftView(id, 1), disabled: i >= layouts.doc.views.length - 1 },
    {
      label: 'Supprimer…',
      danger: true,
      onselect: async () => {
        if (!(await confirmAction({ title: `Supprimer l’affichage « ${v.name} » ?`, message: 'Ses terminaux sont fermés. Les distributions ne sont pas touchées.', confirmLabel: 'Supprimer', danger: true }))) return
        for (const s of layoutFor(viewKey(id)).stacks) for (const b of s.blocks) if (b.type === 'terminal') void invoke('pty_close', { id: b.id }).catch(() => {})
        go('overview')
        deleteView(id)
      },
    },
  ]
}
