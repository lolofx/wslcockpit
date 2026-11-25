// Traduction réactive : t("espace.cle", { var }) relit la langue courante ($state), donc l'interface change à chaud.
import { invoke } from '@tauri-apps/api/core'
import { detectLang, flatten, isLang, translate, type Dict, type Lang, type Params, type Tree } from './core.ts'

// Un fichier JSON par langue et par espace de noms : locales/<lang>/<espace>.json -> clés « <espace>.… ».
function load(lang: Lang): Dict {
  const mods = (lang === 'fr'
    ? import.meta.glob('./locales/fr/*.json', { eager: true, import: 'default' })
    : import.meta.glob('./locales/en/*.json', { eager: true, import: 'default' })) as Record<string, Tree>
  const out: Dict = {}
  for (const [path, tree] of Object.entries(mods)) {
    const ns = path.split('/').pop()!.replace(/\.json$/, '')
    flatten(tree, ns, out)
  }
  return out
}

const dicts = { fr: load('fr'), en: load('en') }

let current = $state<Lang>(detectLang(typeof navigator === 'undefined' ? undefined : navigator.language))

export function t(key: string, params?: Params): string {
  return translate(dicts, current, key, params)
}

export function lang(): Lang {
  return current
}

function syncTray(): void {
  invoke('tray_labels', { open: t('tray.open'), quit: t('tray.quit') }).catch(() => {})
}

/** Lit la langue enregistrée (app.json) ; à défaut, celle du système. */
export async function initLang(): Promise<void> {
  try {
    const saved = await invoke<string>('app_language')
    if (isLang(saved)) current = saved
  } catch {
    /* hors Tauri : langue du navigateur */
  }
  document.documentElement.lang = current
  syncTray()
}

/** Change la langue à chaud et l'enregistre (le tray suit côté Rust). */
export async function setLang(l: Lang): Promise<void> {
  current = l
  document.documentElement.lang = l
  try {
    await invoke('app_set_language', { lang: l })
  } catch {
    /* ignoré : la langue reste appliquée pour la session */
  }
  syncTray()
}
