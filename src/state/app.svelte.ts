import { t } from '../i18n/index.svelte'
// État partagé de l'application : distros, alias, politiques, monitoring, actions.
// Tous les écrans et blocs le lisent ; aucun ne fait ses propres appels de liste.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { confirmAction, promptText, toast } from '../ui/feedback.svelte'
import { addBlock, makeBlock, type Block } from '../workspace/model'
import { loadIcons } from './icons.svelte'
import { loadLaunchers } from './launchers.svelte'
import { layoutFor, loadLayouts, setLayoutExternal } from './layouts.svelte'

export type Distro = { name: string; state: string; version: number; is_default: boolean; vhdx_bytes: number | null }
export type Mon = { ram_bytes: number; cpu_percent: number | null; disks: Record<string, [number, number]> }
export type OpsRequest = { mode: 'export' | 'import' | 'clone' | 'convert' | 'sparse' | 'create' | 'wslupdate'; source: string; version?: number }

export type Tone = 'info' | 'success' | 'error'
export type LogEntry = { id: number; at: number; tone: Tone; text: string; distro?: string }

export const app = $state({
  log: [] as LogEntry[],
  unread: 0,
  opStage: null as string | null,
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

let logId = 0
async function osNotify(text: string) {
  try {
    if (!(await isPermissionGranted()) && (await requestPermission()) !== 'granted') return
    sendNotification({ title: 'WSLCockpit', body: text })
  } catch {
    /* pas de notification système disponible : le journal suffit */
  }
}

// Une seule voie pour informer : journal (centre de notifications), toast, et notification Windows
// quand la fenêtre n'est pas au premier plan (opérations longues terminées).
export function notify(text: string, tone: Tone = 'info', o: { distro?: string; toast?: boolean; system?: boolean } = {}) {
  app.log = [{ id: ++logId, at: Date.now(), tone, text, distro: o.distro }, ...app.log].slice(0, 200)
  app.unread += 1
  if (o.toast !== false) toast(text, tone)
  if (o.system && !document.hasFocus()) void osNotify(text)
}

export const display = (name: string) => app.aliases[name] ?? name
export const find = (name: string) => app.distros.find((d) => d.name === name)

export const size = (b: number) =>
  b >= 1024 ** 3 ? t('shell.size.gb', { n: (b / 1024 ** 3).toFixed(1) }) : t('shell.size.mb', { n: Math.round(b / 1024 ** 2) })
export const stateLabel = (s: string) =>
  s === 'running' ? t('shell.state.running') : s === 'stopped' ? t('shell.state.stopped') : s === 'installing' ? t('shell.state.installing') : s

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
    notify(String(e), 'error', { distro: name === '*' ? undefined : name })
  }
  app.busy = null
  await refresh()
}

export type Cmd = 'start' | 'stop' | 'restart' | 'terminal'
const verb = (cmd: Cmd, name: string) =>
  cmd === 'start' ? t('notif.started', { name }) : cmd === 'stop' ? t('notif.stopped', { name }) : cmd === 'restart' ? t('notif.restarted', { name }) : t('notif.terminal', { name })
export const act = (cmd: Cmd, name: string) =>
  run(name, () => invoke(cmd, { name }), async () => {
    if (cmd === 'start' || cmd === 'restart') await new Promise((r) => setTimeout(r, 1500))
    notify(verb(cmd, display(name)), 'success', { distro: name, toast: false })
  })

// Navigation fournie par la coquille (évite un import circulaire).
export const nav = { go: (_view: string) => {} }

// Terminal intégré : va sur l'espace de la distro ; ajoute un bloc terminal WSL s'il n'y en a pas.
export function openTerminal(name: string) {
  const l = layoutFor(name)
  if (!l.stacks.some((s) => s.blocks.some((b) => b.type === 'terminal'))) {
    setLayoutExternal(name, addBlock(l, makeBlock('terminal', { kind: 'wsl' })))
  }
  nav.go(name)
}

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
    title: t('app.alias.title', { name }),
    message: t('app.alias.message'),
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
  const list = running.length ? running.join(', ') : t('app.shutdown.none')
  if (!(await confirmAction({ title: t('app.shutdown.title'), message: t('app.shutdown.message', { list }), confirmLabel: t('app.shutdown.confirm'), danger: true }))) return
  await run('*', () => invoke('shutdown'), async () => notify(t('notif.shutdownDone'), 'success'))
}

export async function toggleAutostart() {
  try {
    app.autostart = await invoke<boolean>('autostart_set', { on: !app.autostart })
  } catch (e) {
    app.error = String(e)
  }
}

type OpStatus = { active: boolean; kind: string; stage: string; done: string | null; error: string | null; bytes: number }
let opWasActive = false
// Surveille l'opération longue (export, import, clone, conversion) même si son dialogue est fermé.
async function pollOp() {
  try {
    const st = await invoke<OpStatus>('op_status')
    if (opWasActive && !st.active) {
      if (st.error) notify(st.error, 'error', { system: true })
      else if (st.done) notify(st.done, 'success', { system: true })
      refresh()
    }
    opWasActive = st.active
    app.opStage = st.active ? st.stage : null
  } catch {
    /* sans opération : rien à faire */
  }
}

// Un bloc détaché revient dans la disposition de sa distro quand sa fenêtre se ferme.
function listenReattach() {
  const un = listen<{ distro: string; block: Block }>('block-reattach', (e) => {
    const { distro, block } = e.payload
    setLayoutExternal(distro, addBlock(layoutFor(distro), block))
  })
  return () => void un.then((f) => f())
}

// À appeler une fois au démarrage ; renvoie la fonction d'arrêt des minuteries.
// `light` : fenêtre de bloc détaché (liste et monitoring seulement, pas de politiques ni de MAJ).
export function startApp(light = false): () => void {
  if (!light) { loadLayouts(); loadIcons(); loadLaunchers() }
  refresh()
  pollMonitor()
  const tick = setInterval(() => !app.busy && refresh(), 3000)
  const m = setInterval(pollMonitor, 5000)
  if (light) {
    reloadAliases()
    return () => {
      clearInterval(tick)
      clearInterval(m)
    }
  }
  reloadAliases()
  invoke<Record<string, string>>('policies').then((p) => (app.policies = p))
  invoke('apply_policies').catch(() => {})
  invoke<boolean>('autostart_get').then((v) => (app.autostart = v))
  invoke<{ version: string; url: string } | null>('check_update')
    .then((u) => {
      app.update = u
      if (u) notify(t('notif.update', { version: u.version }), 'info', { toast: false })
    })
    .catch(() => {})
  invoke<{ latest: string | null; newer: boolean }>('wsl_update_check')
    .then((u) => u.newer && notify(t('notif.wslUpdate', { latest: String(u.latest) }), 'info', { toast: false }))
    .catch(() => {})
  const o = setInterval(pollOp, 2000)
  const stopReattach = listenReattach()
  return () => {
    clearInterval(tick)
    clearInterval(m)
    clearInterval(o)
    stopReattach()
  }
}
