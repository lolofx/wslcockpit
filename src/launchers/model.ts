// Lanceurs : un outil à ouvrir dans un terminal (Claude Code, Codex, aspire run...). Pur, testable.
import { parseSpec, type IconSpec } from '../icons/model.ts'

export type Launcher = {
  id: string
  name: string
  icon?: IconSpec
  target: 'wsl' | 'windows'
  distro?: string // wsl : une distro précise ; absent = celle du bloc / à choisir
  command: string
  cwd?: string // dossier de départ ; absent = dossier du projet de l'affichage
}

export type LaunchersDoc = { version: 1; launchers: Launcher[] }
export const emptyLaunchers = (): LaunchersDoc => ({ version: 1, launchers: [] })

let n = 0
export const launcherId = () => `l${Date.now().toString(36)}${(n++).toString(36)}`

export function parseLaunchers(raw: unknown): LaunchersDoc {
  const doc = emptyLaunchers()
  const arr = (raw as { launchers?: unknown } | null)?.launchers
  const seen = new Set<string>()
  for (const v of Array.isArray(arr) ? arr : []) {
    const o = v as Partial<Launcher> | null
    if (!o || typeof o.id !== 'string' || !o.id || seen.has(o.id)) continue
    if (typeof o.name !== 'string' || !o.name.trim() || typeof o.command !== 'string' || !o.command.trim()) continue
    if (o.target !== 'wsl' && o.target !== 'windows') continue
    seen.add(o.id)
    const icon = parseSpec(o.icon)
    doc.launchers.push({
      id: o.id,
      name: o.name.trim(),
      target: o.target,
      command: o.command.trim(),
      ...(icon ? { icon } : {}),
      ...(o.target === 'wsl' && typeof o.distro === 'string' && o.distro ? { distro: o.distro } : {}),
      ...(typeof o.cwd === 'string' && o.cwd.trim() ? { cwd: o.cwd.trim() } : {}),
    })
  }
  return doc
}

export function upsertLauncher(doc: LaunchersDoc, l: Launcher): LaunchersDoc {
  const clean = parseLaunchers({ launchers: [l] }).launchers[0]
  if (!clean) return doc
  const has = doc.launchers.some((x) => x.id === clean.id)
  return { ...doc, launchers: has ? doc.launchers.map((x) => (x.id === clean.id ? clean : x)) : [...doc.launchers, clean] }
}

export const removeLauncher = (doc: LaunchersDoc, id: string): LaunchersDoc => ({ ...doc, launchers: doc.launchers.filter((l) => l.id !== id) })

// Où un lanceur est proposé : `current` = distro de l'espace ouvert, null dans un affichage (toute distro).
export const availableIn = (l: Launcher, current: string | null) =>
  l.target === 'windows' || !l.distro || current === null || l.distro === current

export type ProjectRootLike = { distro: string; path: string }

// Props du bloc terminal à créer. `distro` : celle du bloc (ignorée pour Windows).
export function launchProps(l: Launcher, distro: string, root?: ProjectRootLike): Record<string, unknown> {
  if (l.target === 'windows') {
    const dir = l.cwd || (root && !root.distro ? root.path : undefined)
    return { kind: 'powershell', start: 'profile', launch: l.command, title: l.name, ...(dir ? { dir } : {}) }
  }
  const d = l.distro || distro
  const cwd = l.cwd || (root && root.distro === d ? root.path : undefined)
  return { kind: 'wsl', distro: d, launch: l.command, title: l.name, ...(cwd ? { cwd } : {}) }
}

// Outils connus que l'appli sait repérer dans une distro (commande trouvée dans le PATH du shell de connexion).
export const KNOWN_TOOLS: Record<string, string> = { claude: 'Claude Code', codex: 'Codex' }

// Suggestions pour les outils trouvés dans une distro, sauf ceux déjà configurés pour elle.
export function suggestions(doc: LaunchersDoc, distro: string, found: string[]): { tool: string; name: string }[] {
  return found
    .filter((t) => t in KNOWN_TOOLS)
    .filter((t) => !doc.launchers.some((l) => l.target === 'wsl' && l.command.split(/\s+/)[0] === t && (!l.distro || l.distro === distro)))
    .map((t) => ({ tool: t, name: KNOWN_TOOLS[t] }))
}
