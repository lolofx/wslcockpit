// Sélecteur d'icône à la demande : `await pickIcon(...)` ouvre la boîte (hébergée par IconPicker.svelte).
// Résultat : une IconSpec, `null` = retour automatique (initiales), `undefined` = annulé.
import type { IconSpec } from './model'

export type PickAsk = { title: string; label: string; current: IconSpec | null; suggestion?: string | null; done: (r: IconSpec | null | undefined) => void }
export const picker = $state<{ ask: PickAsk | null }>({ ask: null })

export function pickIcon(o: { title: string; label: string; current: IconSpec | null; suggestion?: string | null }): Promise<IconSpec | null | undefined> {
  return new Promise((resolve) => {
    picker.ask = { ...o, done: (r) => { picker.ask = null; resolve(r) } }
  })
}
