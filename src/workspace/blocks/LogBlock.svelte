<script lang="ts">
  import type { BlockContext } from '../registry'
  import { app } from '../../state/app.svelte'
  import { t, lang } from '../../i18n/index.svelte'
  let { distro }: BlockContext = $props()
  const entries = $derived(app.log.filter((e) => !e.distro || e.distro === distro))
  const hm = (ms: number) => new Date(ms).toLocaleTimeString(lang() === 'fr' ? 'fr-FR' : 'en-GB', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
</script>

<div class="log">
  {#if entries.length === 0}
    <p class="empty">{t('blocks.log.empty')}</p>
  {:else}
    <ul>
      {#each entries as e (e.id)}
        <li class={e.tone}><time>{hm(e.at)}</time><span>{e.text}</span></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .log { padding: var(--sp-2) var(--sp-3); font-size: var(--fs-sm); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--sp-1); }
  li { display: flex; gap: var(--sp-2); border-left: 3px solid var(--border); padding-left: var(--sp-2); }
  li.success { border-color: var(--success); }
  li.error { border-color: var(--danger); }
  time { color: var(--text-dim); flex: none; font-variant-numeric: tabular-nums; }
  span { overflow-wrap: anywhere; white-space: pre-wrap; }
  .empty { color: var(--text-dim); margin: 0; }
</style>
