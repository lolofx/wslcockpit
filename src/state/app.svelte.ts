// État partagé de l'application : distros, alias, politiques, monitoring, actions.
// Tous les écrans et blocs le lisent ; aucun ne fait ses propres appels de liste.
import { invoke } from '@tauri-apps/api/core'
import { confirmAction, promptText, toast } from '../ui/feedback.svelte'

export type Distro = { name: string; state: string; version: number; is_default: boolean; vhdx_bytes: number | null }
export type Mon = { ram_bytes: number; cpu_percent: number | null; disks: Record<string, [number, number]> }
export type OpsRequest = { mode: 'export' | 'import' | 'clone' | 'convert' | 'sparse'; source: string; version?: number }

export const app = $state({
  distros: [] as Distro[],
  aliases: {} as Record<string, string>,
  policies: {} as Record<string, string>,
  mon: null as Mon | null,
  update: null as { version: string; url: string } | null,
  autostart: false,
  error: '',
  loaded: false,
  busy: null as string | null,
  ops: null as OpsRequest | null,
  deleting: null as string | null,
})

export const display = (name: string) => app.aliases[name] ?? name
export const find = (name: string) => app.distros.find((d) => d.name === name)

export const size = (b: number) => (b >= 1024 ** 3 ? `${(b / 1024 ** 3).toFixed(1)} Go` : `${Math.round(b / 1024 ** 2)} Mo`)
export const stateLabel = (s: string) =>
  s === 'running' ? 'En marche' : s === 'stopped' ? 'Arrêtée' : s === 'installing' ? 'Installation' : s

export async function refresh() {
  try {
    app.distros = await invoke<Distro[]>('distros')
    app.error = ''
  } catch (e) {
    app.error = String(e)
  } finally {
    app.loaded = true
  }
}

async function pollMonitor() {
  if (document.hidden) return
  try {
    app.mon = await invoke<Mon | null>('monitor')
  } catch {
    app.mon = null
  }
}

export const reloadAliases = () => invoke<Record<string, string>>('aliases').then((a) => (app.aliases = a))

async function run(name: string, fn: () => Promise<unknown>, after?: () => Promise<void>) {
  app.busy = name
  try {
    await fn()
    app.error = ''
    if (after) await after()
  } catch (e) {
    app.error = String(e)
    toast(String(e), 'error')
  }
  app.busy = null
  await refresh()
}

export type Cmd = 'start' | 'stop' | 'restart' | 'terminal'
export const act = (cmd: Cmd, name: string) =>
  run(name, () => invoke(cmd, { name }), async () => {
    if (cmd === 'start' || cmd === 'restart') await new Promise((r) => setTimeout(r, 1500))
  })

export const makeDefault = (name: string) => run(name, () => invoke('set_default_distro', { name }))

export async function setPolicy(name: string, policy: string) {
  try {
    app.policies = await invoke<Record<string, string>>('set_policy', { name, policy })
    if (policy === 'keepalive') await act('start', name)
  } catch (e) {
    app.error = String(e)
  }
}

export async function rename(name: string) {
  const v = await promptText({
    title: `Alias pour « ${name} »`,
    message: "Vide = retirer l'alias. Affichage seulement, WSL n'est pas modifié.",
    value: app.aliases[name] ?? '',
  })
  if (v === null) return
  try {
    app.aliases = await invoke<Record<string, string>>('set_alias', { name, alias: v })
  } catch (e) {
    app.error = String(e)
  }
}

export async function shutdownAll() {
  const running = app.distros.filter((d) => d.state === 'running').map((d) => d.name)
  const list = running.length ? running.join(', ') : 'aucune distribution en marche'
  if (!(await confirmAction({ title: 'Tout arrêter ?', message: `wsl --shutdown\n\nEn marche : ${list}`, confirmLabel: 'Tout arrêter', danger: true }))) return
  await run('*', () => invoke('shutdown'), async () => toast('Toutes les distributions sont arrêtées.', 'success'))
}

export async function toggleAutostart() {
  try {
    app.autostart = await invoke<boolean>('autostart_set', { on: !app.autostart })
  } catch (e) {
    app.error = String(e)
  }
}

// À appeler une fois au démarrage ; renvoie la fonction d'arrêt des minuteries.
export function startApp(): () => void {
  reloadAliases()
  invoke<Record<string, string>>('policies').then((p) => (app.policies = p))
  invoke('apply_policies').catch(() => {})
  invoke<boolean>('autostart_get').then((v) => (app.autostart = v))
  refresh()
  pollMonitor()
  invoke<{ version: string; url: string } | null>('check_update').then((u) => (app.update = u)).catch(() => {})
  const t = setInterval(() => !app.busy && refresh(), 3000)
  const m = setInterval(pollMonitor, 5000)
  return () => {
    clearInterval(t)
    clearInterval(m)
  }
}
