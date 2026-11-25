<script lang="ts">
  import { t, lang } from '../i18n/index.svelte'
  import Dialog from '../ui/Dialog.svelte'
  import Button from '../ui/Button.svelte'
  import { app, display } from '../state/app.svelte'
  let { onclose }: { onclose: () => void } = $props()
  const when = (ts: number) => new Date(ts).toLocaleString(lang() === 'fr' ? 'fr-FR' : 'en-GB', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
</script>

<Dialog title={t('shell.notifications.title')} {onclose} width={560}>
  {#if app.log.length === 0}
    <p class="empty">{t('shell.notifications.empty')}</p>
  {:else}
    <ul>
      {#each app.log as e (e.id)}
        <li class={e.tone}>
          <time>{when(e.at)}</time>
          <span>{#if e.distro}<b>{display(e.distro)}</b> {/if}{e.text}</span>
        </li>
      {/each}
    </ul>
    <div class="foot"><Button onclick={() => (app.log = [])}>{t('shell.notifications.clear')}</Button></div>
  {/if}
</Dialog>

<style>
  ul { list-style: none; margin: 0 0 var(--sp-3); padding: 0; display: grid; gap: var(--sp-2); max-height: 50vh; overflow: auto; }
  li { display: flex; gap: var(--sp-3); border-left: 3px solid var(--border); padding-left: var(--sp-2); font-size: var(--fs-sm); }
  li.success { border-color: var(--success); }
  li.error { border-color: var(--danger); }
  time { flex: none; color: var(--text-dim); }
  span { overflow-wrap: anywhere; white-space: pre-wrap; }
  .empty { color: var(--text-dim); }
</style>
