<script lang="ts">
  import Dialog from '../ui/Dialog.svelte'
  import Button from '../ui/Button.svelte'
  import Icon from './Icon.svelte'
  import { t } from '../i18n/index.svelte'
  import { picker } from './picker.svelte'
  import { LOGOS, SYMBOLS } from './data'
  import { EMOJI } from './emoji'
  import { cleanInitials, initialsOf, type IconKind, type IconSpec } from './model'

  const COLORS = ['#e95420', '#2f81f7', '#3fb950', '#d29922', '#f85149', '#a371f7', '#db61a2', '#8b949e']
  const TABS = $derived<[IconKind, string][]>([['logo', t('icons.tabs.logo')], ['symbol', t('icons.tabs.symbol')], ['emoji', t('icons.tabs.emoji')], ['initials', t('icons.tabs.initials')]])

  const ask = $derived(picker.ask)
  let tab = $state<IconKind>('logo')
  let query = $state('')
  let color = $state('')
  let initials = $state('')
  let seeded = $state<object | null>(null)

  // Réinitialise l'état local à chaque ouverture.
  $effect(() => {
    if (ask && seeded !== ask) {
      seeded = ask
      tab = ask.current?.kind ?? 'logo'
      query = ''
      color = ask.current?.color ?? ''
      initials = ask.current?.kind === 'initials' ? ask.current.value : initialsOf(ask.label)
    }
  })

  const q = $derived(query.trim().toLowerCase())
  const logos = $derived(Object.entries(LOGOS).filter(([k, v]) => !q || k.includes(q) || v.label.toLowerCase().includes(q)))
  const symbols = $derived(Object.keys(SYMBOLS).filter((k) => !q || k.includes(q)))
  const emojis = $derived(EMOJI.filter(([, kw]) => !q || kw.includes(q)))

  const choose = (kind: IconKind, value: string) => ask?.done({ kind, value, ...(color ? { color } : {}) })
  const preview = $derived<IconSpec | null>(ask?.current ?? null)
</script>

{#if ask}
  <Dialog title={ask.title} width={560} onclose={() => ask.done(undefined)}>
    <div class="tabs" role="tablist">
      {#each TABS as [k, l]}
        <button role="tab" aria-selected={tab === k} class:on={tab === k} onclick={() => { tab = k; query = '' }}>{l}</button>
      {/each}
    </div>
    {#if tab !== 'initials'}
      <input class="search" placeholder={t('icons.search')} bind:value={query} aria-label={t('icons.searchLabel')} />
    {/if}
    <div class="grid">
      {#if tab === 'logo'}
        {#each logos as [k, v]}
          <button class="cell" class:sug={k === ask.suggestion} title={v.label} onclick={() => choose('logo', k)}><Icon spec={{ kind: 'logo', value: k, color: color || undefined }} size={28} /></button>
        {/each}
      {:else if tab === 'symbol'}
        {#each symbols as k}
          <button class="cell" title={k} onclick={() => choose('symbol', k)}><Icon spec={{ kind: 'symbol', value: k, color: color || undefined }} size={28} /></button>
        {/each}
      {:else if tab === 'emoji'}
        {#each emojis as [e, kw]}
          <button class="cell" title={kw} onclick={() => choose('emoji', e)}><Icon spec={{ kind: 'emoji', value: e }} size={28} /></button>
        {/each}
      {:else}
        <div class="ini">
          <label>{t('icons.chars')} <input maxlength="4" bind:value={initials} oninput={() => (initials = cleanInitials(initials))} /></label>
          <Icon spec={{ kind: 'initials', value: initials || '?', color: color || undefined }} size={44} />
          <Button variant="primary" disabled={!initials} onclick={() => choose('initials', initials)}>{t('icons.use')}</Button>
        </div>
      {/if}
    </div>
    {#if tab !== 'emoji'}
      <div class="colors" aria-label={t('icons.color')}>
        <span class="dim">{t('icons.color')}</span>
        <button class="sw none" class:on={!color} title={t('icons.default')} onclick={() => (color = '')}>∅</button>
        {#each COLORS as c}
          <button class="sw" class:on={color === c} style:background={c} title={c} aria-label={c} onclick={() => (color = c)}></button>
        {/each}
      </div>
    {/if}
    {#snippet footer()}
      <Button onclick={() => ask.done(null)}>{t('icons.auto')}</Button>
      <Button onclick={() => ask.done(undefined)}>{t('icons.cancel')}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .tabs { display: flex; gap: var(--sp-1); margin-bottom: var(--sp-2); }
  .tabs button { font: inherit; padding: var(--sp-1) var(--sp-3); border: 1px solid transparent; border-radius: var(--radius); background: none; color: var(--text); cursor: pointer; }
  .tabs button.on { background: var(--surface-3); border-color: var(--accent); }
  .search { width: 100%; box-sizing: border-box; font: inherit; padding: var(--sp-2) var(--sp-3); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); margin-bottom: var(--sp-2); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(3rem, 1fr)); gap: var(--sp-1); max-height: 18rem; overflow-y: auto; }
  .cell { display: grid; place-items: center; height: 3rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface-1); color: var(--text); cursor: pointer; }
  .cell:hover, .cell:focus-visible { background: var(--surface-2); outline: none; border-color: var(--accent); }
  .cell.sug { border-color: var(--accent); }
  .ini { display: flex; align-items: center; gap: var(--sp-3); grid-column: 1 / -1; }
  .ini input { width: 4rem; font: inherit; padding: var(--sp-2); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); text-transform: uppercase; }
  .colors { display: flex; align-items: center; gap: var(--sp-2); margin-top: var(--sp-3); }
  .dim { color: var(--text-dim); font-size: var(--fs-sm); }
  .sw { width: 1.4rem; height: 1.4rem; border-radius: 50%; border: 2px solid var(--border); cursor: pointer; color: var(--text-dim); font-size: 0.7rem; padding: 0; }
  .sw.on { border-color: var(--text); }
  .sw.none { background: none; }
</style>
