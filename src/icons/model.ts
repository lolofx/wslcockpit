// Icônes : pur (sans Svelte ni Tauri), donc testable avec `node --test`.
// Une icône choisie est une IconSpec ; sans choix, on affiche 2 initiales calculées depuis le nom.
export type IconKind = 'logo' | 'symbol' | 'emoji' | 'initials'
export type IconSpec = { kind: IconKind; value: string; color?: string }

const KINDS: IconKind[] = ['logo', 'symbol', 'emoji', 'initials']
const COLOR = /^#[0-9a-fA-F]{6}$/

// Deux initiales : première lettre de deux mots, sinon deux premiers caractères. Majuscules.
export function initialsOf(label: string): string {
  const words = label.split(/[^\p{L}\p{N}]+/u).filter(Boolean)
  const chars = (s: string) => [...s]
  let r: string[]
  if (words.length >= 2) r = [chars(words[0])[0], chars(words[1])[0]]
  else r = chars(words[0] ?? label.trim()).slice(0, 2)
  return r.join('').toUpperCase()
}

export function cleanInitials(s: string): string {
  return [...s.trim()].slice(0, 2).join('').toUpperCase()
}

export function parseSpec(raw: unknown): IconSpec | null {
  if (!raw || typeof raw !== 'object') return null
  const o = raw as Record<string, unknown>
  if (!KINDS.includes(o.kind as IconKind) || typeof o.value !== 'string' || !o.value) return null
  const spec: IconSpec = { kind: o.kind as IconKind, value: o.kind === 'initials' ? cleanInitials(o.value) : o.value }
  if (!spec.value) return null
  if (typeof o.color === 'string' && COLOR.test(o.color)) spec.color = o.color
  return spec
}

export type IconsDoc = { version: 1; distros: Record<string, IconSpec> }
export const emptyIcons = (): IconsDoc => ({ version: 1, distros: {} })

export function parseIcons(raw: unknown): IconsDoc {
  const doc = emptyIcons()
  const d = (raw as { distros?: unknown } | null)?.distros
  if (d && typeof d === 'object') {
    for (const [k, v] of Object.entries(d as Record<string, unknown>)) {
      const s = parseSpec(v)
      if (s) doc.distros[k] = s
    }
  }
  return doc
}

export function setDistroIcon(doc: IconsDoc, name: string, spec: IconSpec | null): IconsDoc {
  const distros = { ...doc.distros }
  if (spec) distros[name] = spec
  else delete distros[name]
  return { ...doc, distros }
}

// Icône à afficher : le choix de l'utilisateur, sinon les initiales du libellé.
export const resolveIcon = (spec: IconSpec | null | undefined, label: string): IconSpec =>
  spec ?? { kind: 'initials', value: initialsOf(label) }

// Logo suggéré d'après le nom de la distribution (Ubuntu-24.04, kali-linux, Debian...).
const GUESS: [RegExp, string][] = [
  [/ubuntu/i, 'ubuntu'], [/debian/i, 'debian'], [/kali/i, 'kalilinux'], [/fedora/i, 'fedora'],
  [/arch/i, 'archlinux'], [/alpine/i, 'alpinelinux'], [/opensuse|suse/i, 'opensuse'], [/mint/i, 'linuxmint'],
  [/rocky/i, 'rockylinux'], [/alma/i, 'almalinux'], [/centos/i, 'centos'], [/rhel|redhat/i, 'redhat'],
  [/nixos/i, 'nixos'], [/gentoo/i, 'gentoo'], [/manjaro/i, 'manjaro'], [/pop/i, 'popos'], [/void/i, 'voidlinux'],
]
export function guessLogo(distro: string): string | null {
  return GUESS.find(([re]) => re.test(distro))?.[1] ?? null
}
