<script lang="ts">
  import { t, bm, lang, setLang } from '../i18n/index.svelte'
  import { LANGS, type Lang } from '../i18n/core.ts'
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';

  // `fixed` : distro imposée (bloc d'espace de travail) ; sans lui, onglets Global + distros.
  let { distros, onclose, fixed }: { distros: string[]; onclose?: () => void; fixed?: string } = $props();

  type FieldDef = { id: string; label: string; hint: string; options?: string[] };
  const GLOBAL = $derived<FieldDef[]>([
    { id: 'memory', label: t('config.fields.memory.label'), hint: t('config.fields.memory.hint') },
    { id: 'processors', label: t('config.fields.processors.label'), hint: t('config.fields.processors.hint') },
    { id: 'swap', label: t('config.fields.swap.label'), hint: t('config.fields.swap.hint') },
    { id: 'networkingMode', label: t('config.fields.networkingMode.label'), hint: t('config.fields.networkingMode.hint'), options: ['', 'nat', 'mirrored', 'bridged', 'virtioproxy', 'none'] },
    { id: 'vmIdleTimeout', label: t('config.fields.vmIdleTimeout.label'), hint: t('config.fields.vmIdleTimeout.hint') }
  ]);
  const DISTRO = $derived<FieldDef[]>([
    { id: 'systemd', label: t('config.fields.systemd.label'), hint: t('config.fields.systemd.hint'), options: ['', 'true', 'false'] },
    { id: 'defaultUser', label: t('config.fields.defaultUser.label'), hint: t('config.fields.defaultUser.hint') },
    { id: 'automountEnabled', label: t('config.fields.automountEnabled.label'), hint: t('config.fields.automountEnabled.hint'), options: ['', 'true', 'false'] }
  ]);

  let target = $state<string | null>(fixed ?? null); // null = global
  let values = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let message = $state('');
  let failure = $state('');
  let needStart = $state(false);
  let loaded = $state(false);
  let saved = $state(false);

  const fields = $derived(target === null ? GLOBAL : DISTRO);

  async function load(allowStart = false) {
    loaded = false;
    errors = {};
    message = '';
    failure = '';
    saved = false;
    needStart = false;
    try {
      const v = await invoke<{ values: [string, string][] }>('config_load', { distro: target, allowStart });
      values = Object.fromEntries(v.values);
      loaded = true;
    } catch (e) {
      if (String(e) === 'NEEDS_START') needStart = true;
      else failure = String(e);
    }
  }

  function pick(tg: string | null) {
    target = tg;
    load();
  }

  async function save() {
    errors = {};
    failure = '';
    saved = false;
    try {
      await invoke('config_save', { distro: target, values: Object.entries(values), allowStart: true });
      saved = true;
    } catch (e) {
      const s = String(e);
      if (s.startsWith('INVALID:')) {
        errors = Object.fromEntries(JSON.parse(s.slice(8)) as [string, string][]);
      } else failure = s;
    }
  }

  let base = $state('')
  let baseMsg = $state('')
  async function saveBase() {
    try {
      base = await invoke<string>('op_set_default_base', { path: base })
      baseMsg = t('config.saved')
    } catch (e) {
      baseMsg = String(e)
    }
  }
  async function browseBase() {
    const r = await openDialog({ multiple: false, directory: true, defaultPath: base || undefined })
    if (typeof r === 'string') {
      base = r
      await saveBase()
    }
  }

  onMount(() => {
    invoke<string>('op_default_base').then((b) => (base = b))
    load();
  });
</script>

<section class="panel">
  {#if fixed === undefined}
    <div class="top">
      <h2>{t('config.title')}</h2>
      {#if onclose}<button onclick={onclose}>{t('config.back')}</button>{/if}
    </div>
    <div class="tabs">
      <button class:active={target === null} onclick={() => pick(null)}>{t('config.tabGlobal')}</button>
      {#each distros as d (d)}
        <button class:active={target === d} onclick={() => pick(d)}>{d}</button>
      {/each}
    </div>
  {/if}

  {#if target === null}
    <div class="row">
      <span>{t('config.language.label')}</span>
      <select value={lang()} onchange={(e) => setLang(e.currentTarget.value as Lang)}>
        {#each LANGS as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
      </select>
      <span class="muted">{t('config.language.hint')}</span>
    </div>
    <div class="row">
      <span>{t('config.base.label')}</span>
      <div class="pick">
        <input bind:value={base} placeholder="C:\\WSL" onchange={saveBase} />
        <button onclick={browseBase}>{t('config.base.browse')}</button>
      </div>
      <span class="muted">{t('config.base.hint')}</span>
      {#if baseMsg}<span class="muted">{bm(baseMsg)}</span>{/if}
    </div>
    <p class="muted">{t('config.globalFile')}</p>
  {:else}
    <p class="muted">{t('config.distroFile')}</p>
  {/if}

  {#if needStart}
    <div class="warn">
      {t('config.needStart', { name: target ?? '' })}
      <button onclick={() => load(true)}>{t('config.startRead')}</button>
    </div>
  {:else if loaded}
    {#each fields as f (f.id)}
      <label class="row">
        <span>{f.label}</span>
        {#if f.options}
          <select bind:value={values[f.id]}>
            {#each f.options as o}<option value={o}>{o === '' ? t('config.unset') : o}</option>{/each}
          </select>
        {:else}
          <input bind:value={values[f.id]} placeholder={f.hint} class:bad={errors[f.id]} />
        {/if}
        {#if errors[f.id]}<span class="err">{bm(errors[f.id])}</span>{/if}
      </label>
    {/each}
    <div class="foot">
      <button class="primary" onclick={save}>{t('config.save')}</button>
      <span class="muted">{t('config.backupNote')}</span>
    </div>
    {#if saved}
      <div class="warn">
        {t('config.restart')}
      </div>
    {/if}
  {:else if !failure}
    <p class="muted">{t('config.load')}</p>
  {/if}
  {#if failure}<div class="error">{bm(failure)}</div>{/if}
</section>

<style>
  .top { display: flex; justify-content: space-between; align-items: center; }
  h2 { font-size: 1.15rem; margin: 0; }
  .tabs { display: flex; flex-wrap: wrap; gap: var(--sp-1); margin: var(--sp-3) 0; padding: var(--sp-1); background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-lg); width: fit-content; max-width: 100%; }
  .row { display: grid; grid-template-columns: 14rem 1fr; gap: 0.2rem 0.8rem; align-items: center; margin-bottom: 0.6rem; }
  .pick { display: flex; gap: 0.4rem; }
  .pick input { flex: 1; min-width: 0; }
  .row > .muted { grid-column: 2; }
  .row :global(.bad) { border-color: var(--danger); }
  .err { grid-column: 2; color: var(--danger); font-size: 0.85rem; }
  .foot { display: flex; align-items: center; gap: 0.8rem; margin-top: 1rem; }
  .muted { color: var(--text-dim); font-size: var(--fs-sm); }
</style>
