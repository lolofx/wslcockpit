// Noyau i18n pur (sans Svelte ni Tauri) : testable avec node --test.
// Clés hiérarchiques « espace.chemin.cle », valeurs texte avec {{variable}},
// pluriel par objet { one, other } (+ zero/few/many selon Intl.PluralRules), repli sur le français puis sur la clé.

export type Lang = 'fr' | 'en'
export const LANGS: { id: Lang; label: string }[] = [
  { id: 'fr', label: 'Français' },
  { id: 'en', label: 'English' },
]
export const FALLBACK: Lang = 'fr'

export type Params = Record<string, string | number>
export type Leaf = string | Record<string, string>
export type Tree = { [k: string]: Tree | Leaf }
export type Dict = Record<string, Leaf>

const PLURAL_KEYS = new Set(['zero', 'one', 'two', 'few', 'many', 'other'])

function isPlural(o: Tree | Leaf): o is Record<string, string> {
  if (typeof o !== 'object') return false
  const ks = Object.keys(o)
  return ks.length > 0 && ks.every((k) => PLURAL_KEYS.has(k) && typeof (o as Record<string, unknown>)[k] === 'string')
}

/** Aplatit { shell: { title: "x" } } en { "shell.title": "x" }. */
export function flatten(tree: Tree, prefix = '', out: Dict = {}): Dict {
  for (const [k, v] of Object.entries(tree)) {
    const key = prefix ? `${prefix}.${k}` : k
    if (typeof v === 'string' || isPlural(v)) out[key] = v as Leaf
    else flatten(v as Tree, key, out)
  }
  return out
}

export function interpolate(text: string, params?: Params): string {
  if (!params) return text
  return text.replace(/\{\{\s*(\w+)\s*\}\}/g, (m, name: string) => (name in params ? String(params[name]) : m))
}

export function translate(dicts: Record<Lang, Dict>, lang: Lang, key: string, params?: Params): string {
  const leaf = dicts[lang]?.[key] ?? dicts[FALLBACK]?.[key]
  if (leaf === undefined) return key
  if (typeof leaf === 'string') return interpolate(leaf, params)
  const n = Number(params?.count ?? 0)
  const cat = new Intl.PluralRules(lang).select(n)
  const text = (n === 0 && leaf.zero) || leaf[cat] || leaf.other || Object.values(leaf)[0]
  return interpolate(text, params)
}

/** Langue du système (navigator.language) ramenée aux langues connues. */
export function detectLang(nav: string | undefined): Lang {
  const l = (nav ?? '').toLowerCase().slice(0, 2)
  return l === 'fr' ? 'fr' : l === 'en' ? 'en' : FALLBACK
}

export function isLang(v: unknown): v is Lang {
  return v === 'fr' || v === 'en'
}

/** Clés présentes dans a mais pas dans b. */
export function missingKeys(a: Dict, b: Dict): string[] {
  return Object.keys(a).filter((k) => !(k in b))
}

/** Variables {{x}} d'une valeur (toutes variantes de pluriel confondues). */
export function varsOf(leaf: Leaf): string[] {
  const texts = typeof leaf === 'string' ? [leaf] : Object.values(leaf)
  const s = new Set<string>()
  for (const t of texts) for (const m of t.matchAll(/\{\{\s*(\w+)\s*\}\}/g)) s.add(m[1])
  return [...s].sort()
}
