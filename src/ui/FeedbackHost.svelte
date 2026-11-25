<script lang="ts">
  import { t as tr } from '../i18n/index.svelte'
  import { feedback, dismiss } from './feedback.svelte'
  import Dialog from './Dialog.svelte'
  import Button from './Button.svelte'
  import Field from './Field.svelte'

  let text = $state('')
  $effect(() => { if (feedback.ask?.kind === 'prompt') text = feedback.ask.value })

  function done(v: boolean | string | null) {
    const a = feedback.ask
    feedback.ask = null
    if (!a) return
    if (a.kind === 'confirm') a.resolve(v === true)
    else a.resolve(typeof v === 'string' ? v : null)
  }
</script>

{#if feedback.ask}
  {@const a = feedback.ask}
  <Dialog title={a.title} width={440} onclose={() => done(a.kind === 'confirm' ? false : null)}>
    {#if a.message}<p class="msg">{a.message}</p>{/if}
    {#if a.kind === 'prompt'}
      <form id="ask" onsubmit={(e) => { e.preventDefault(); done(text) }}>
        <Field label={tr('ui.value')} bind:value={text} />
      </form>
    {/if}
    {#snippet footer()}
      <Button variant="ghost" onclick={() => done(a.kind === 'confirm' ? false : null)}>{tr('ui.cancel')}</Button>
      {#if a.kind === 'confirm'}
        <Button variant={a.danger ? 'danger' : 'primary'} onclick={() => done(true)}>{a.confirmLabel}</Button>
      {:else}
        <Button variant="primary" type="submit" form="ask">{tr('ui.ok')}</Button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<div class="toasts" aria-live="polite">
  {#each feedback.toasts as t (t.id)}
    <div class="toast {t.tone}" role={t.tone === 'error' ? 'alert' : 'status'}>
      <span>{t.text}</span>
      <button aria-label={tr('ui.close')} onclick={() => dismiss(t.id)}>✕</button>
    </div>
  {/each}
</div>

<style>
  .msg { margin: 0; white-space: pre-wrap; color: var(--text-dim); }
  .toasts { position: fixed; right: var(--sp-4); bottom: var(--sp-4); z-index: 200; display: grid; gap: var(--sp-2); max-width: min(380px, calc(100vw - 2 * var(--sp-4))); }
  .toast { display: flex; gap: var(--sp-3); align-items: start; justify-content: space-between; padding: var(--sp-3) var(--sp-4); border-radius: var(--radius); background: var(--surface-1); border: 1px solid var(--border); border-left: 4px solid var(--accent); box-shadow: var(--shadow); font-size: var(--fs-sm); white-space: pre-wrap; overflow-wrap: anywhere; }
  .success { border-left-color: var(--success); }
  .error { border-left-color: var(--danger); }
  .toast button { border: 0; background: none; color: var(--text-dim); cursor: pointer; padding: 0; }
</style>
