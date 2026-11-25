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
