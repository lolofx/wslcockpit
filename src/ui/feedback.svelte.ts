import { t } from '../i18n/index.svelte'

// Remplace alert/prompt/confirm : file d'états lue par FeedbackHost.svelte.
export type Toast = { id: number; tone: 'info' | 'success' | 'error'; text: string }
export type Ask =
  | { kind: 'confirm'; title: string; message: string; confirmLabel: string; danger: boolean; resolve: (v: boolean) => void }
  | { kind: 'prompt'; title: string; message: string; value: string; resolve: (v: string | null) => void }

export const feedback = $state<{ toasts: Toast[]; ask: Ask | null }>({ toasts: [], ask: null })

let n = 0
export function toast(text: string, tone: Toast['tone'] = 'info', ms = tone === 'error' ? 8000 : 4000) {
  const id = ++n
  feedback.toasts.push({ id, tone, text })
  setTimeout(() => dismiss(id), ms)
}
export function dismiss(id: number) {
  feedback.toasts = feedback.toasts.filter((t) => t.id !== id)
}

export function confirmAction(o: { title: string; message?: string; confirmLabel?: string; danger?: boolean }): Promise<boolean> {
  return new Promise((resolve) => {
    feedback.ask = { kind: 'confirm', title: o.title, message: o.message ?? '', confirmLabel: o.confirmLabel ?? t('ui.confirm'), danger: o.danger ?? false, resolve }
  })
}

export function promptText(o: { title: string; message?: string; value?: string }): Promise<string | null> {
  return new Promise((resolve) => {
    feedback.ask = { kind: 'prompt', title: o.title, message: o.message ?? '', value: o.value ?? '', resolve }
  })
}
