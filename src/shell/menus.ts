import { t } from '../i18n/index.svelte'
import { chooseDistroIcon } from '../state/icons.svelte'
import { app, act, openTerminal, display, makeDefault, rename, type Distro } from '../state/app.svelte'

export type MenuItem = { label: string; onselect: () => void; danger?: boolean; disabled?: boolean }

// Menu d'une distro : identique sur la carte, dans la barre haute et le rail (une seule définition).
export function distroMenu(d: Distro): MenuItem[] {
  const running = d.state === 'running'
  const busy = app.busy !== null
  const items: MenuItem[] = [
    { label: t('shell.menu.terminal'), onselect: () => openTerminal(d.name) },
    { label: t('shell.menu.terminalExternal'), onselect: () => void act('terminal', d.name), disabled: busy },
    ...(running ? [{ label: t('shell.menu.restart'), onselect: () => void act('restart', d.name), disabled: busy }] : []),
    { label: t('shell.menu.changeIcon'), onselect: () => void chooseDistroIcon(d.name, display(d.name)) },
    { label: t('shell.menu.editAlias'), onselect: () => void rename(d.name) },
    ...(d.is_default ? [] : [{ label: t('shell.menu.makeDefault'), onselect: () => void makeDefault(d.name), disabled: busy }]),
    { label: t('shell.menu.export'), onselect: () => (app.ops = { mode: 'export', source: d.name }), disabled: busy },
    { label: t('shell.menu.clone'), onselect: () => (app.ops = { mode: 'clone', source: d.name }), disabled: busy },
    ...(running
      ? []
      : [
          { label: t('shell.menu.convert', { version: d.version === 2 ? 1 : 2 }), onselect: () => (app.ops = { mode: 'convert', source: d.name, version: d.version }), disabled: busy },
          { label: t('shell.menu.sparse'), onselect: () => (app.ops = { mode: 'sparse', source: d.name }), disabled: busy },
        ]),
    { label: t('shell.menu.delete'), onselect: () => (app.deleting = d.name), danger: true, disabled: busy || d.state === 'installing' },
  ]
  return items
}

export const title = (d: Distro) => display(d.name)

// --- Affichages
import { layouts, layoutFor, createView, patchView, deleteView, shiftView, copyView, findView } from '../state/layouts.svelte'
import { viewKey } from '../workspace/model'
import { promptText, confirmAction, toast } from '../ui/feedback.svelte'
import { pickIcon } from '../icons/picker.svelte'
import { invoke } from '@tauri-apps/api/core'

export async function newView(go: (v: string) => void) {
  const n = await promptText({ title: t('shell.view.newTitle'), message: t('shell.view.newMessage'), value: '' })
  if (n === null) return
  go(viewKey(createView(n.trim() || t('shell.view.defaultName'))))
}

export function viewMenu(id: string, go: (v: string) => void): MenuItem[] {
  const v = findView(id)
  if (!v) return []
  const i = layouts.doc.views.findIndex((x) => x.id === id)
  return [
    {
      label: t('shell.view.rename'),
      onselect: async () => {
        const n = await promptText({ title: t('shell.view.nameTitle'), value: v.name })
        if (n?.trim()) patchView(id, { name: n })
      },
    },
    {
      label: t('shell.menu.changeIcon'),
      onselect: async () => {
        const r = await pickIcon({ title: t('shell.view.iconTitle', { name: v.name }), label: v.name, current: v.icon ?? null })
        if (r !== undefined) patchView(id, { icon: r ?? undefined })
      },
    },
    {
      label: v.root ? t('shell.view.rootWith', { root: `${v.root.distro ? v.root.distro + ':' : ''}${v.root.path}` }) : t('shell.view.root'),
      onselect: async () => {
        const txt = await promptText({
          title: t('shell.view.rootTitle'),
          message: t('shell.view.rootMessage'),
          value: v.root ? (v.root.distro ? `${v.root.distro}:${v.root.path}` : v.root.path) : '',
        })
        if (txt === null) return
        if (!txt.trim()) return patchView(id, { root: undefined })
        const r = await invoke<{ distro: string; path: string } | null>('project_parse', { text: txt })
        if (r) patchView(id, { root: r })
        else toast(t('shell.view.rootUnknown'), 'error')
      },
    },
    { label: t('shell.view.duplicate'), onselect: () => { const c = copyView(id); if (c) go(viewKey(c)) } },
    { label: t('shell.view.up'), onselect: () => shiftView(id, -1), disabled: i <= 0 },
    { label: t('shell.view.down'), onselect: () => shiftView(id, 1), disabled: i >= layouts.doc.views.length - 1 },
    {
      label: t('shell.view.delete'),
      danger: true,
      onselect: async () => {
        if (!(await confirmAction({ title: t('shell.view.deleteTitle', { name: v.name }), message: t('shell.view.deleteMessage'), confirmLabel: t('shell.view.deleteConfirm'), danger: true }))) return
        for (const s of layoutFor(viewKey(id)).stacks) for (const b of s.blocks) if (b.type === 'terminal') void invoke('pty_close', { id: b.id }).catch(() => {})
        go('overview')
        deleteView(id)
      },
    },
  ]
}
