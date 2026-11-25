<script lang="ts">
  import { t, bm } from '../i18n/index.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { open, save } from '@tauri-apps/plugin-dialog';

  let { mode, source, version = 2, onclose }: { mode: 'export' | 'import' | 'clone' | 'convert' | 'sparse' | 'create' | 'wslupdate'; source: string; version?: number; onclose: () => void } = $props();

  type Status = { active: boolean; kind: string; stage: string; done: string | null; error: string | null; bytes: number };

  let base = $state('');
  let name = $state('');
  let path = $state(''); // export : archive .tar ; import/clone : dossier d'installation
  let tar = $state(''); // import : archive source
  let status = $state<Status | null>(null);
  let error = $state('');
  let started = $state(false);
  let sparseReason = $state('');
  // création : catalogue en ligne
  let online = $state<{ name: string; friendly: string }[]>([]);
  let onlineLoading = $state(false);
  let selected = $state('');
  let custom = $state(false);
  let customReason = $state('');
  let upd = $state<{ current: string | null; latest: string | null; newer: boolean } | null>(null);

  const size = (b: number) =>
    b >= 1024 ** 3 ? t('ops.size.gb', { n: (b / 1024 ** 3).toFixed(2) }) : t('ops.size.mb', { n: Math.round(b / 1024 ** 2) });

  const title = $derived(
    mode === 'export'
      ? t('ops.title.export', { name: source })
      : mode === 'clone'
        ? t('ops.title.clone', { name: source })
        : mode === 'convert'
          ? t('ops.title.convert', { name: source })
          : mode === 'sparse'
            ? t('ops.title.sparse', { name: source })
            : mode === 'wslupdate'
              ? t('ops.title.wslupdate')
            : mode === 'create'
              ? t('ops.title.create')
              : t('ops.title.import')
  );

  async function loadOnline() {
    onlineLoading = true;
    error = '';
    try {
      online = await invoke<{ name: string; friendly: string }[]>('list_online');
      if (!selected) selected = online.find((d) => d.name === 'Ubuntu')?.name ?? online[0]?.name ?? '';
    } catch (e) {
      error = String(e);
    }
    onlineLoading = false;
  }

  function pickDistro(n: string) {
    selected = n;
    if (custom) {
      name = n;
      path = base ? `${base}\\${n}` : path;
    }
  }

  function toggleCustom() {
    if (custom) pickDistro(selected);
  }

  onMount(() => {
    if (mode === 'wslupdate') invoke<typeof upd>('wsl_update_check').then((u) => (upd = u));
    if (mode === 'create') {
      loadOnline();
      invoke('custom_install_available').catch((e) => (customReason = String(e)));
    }
    if (mode === 'sparse') invoke('sparse_available').catch((e) => (sparseReason = String(e)));
    invoke<string>('op_default_base').then((b) => {
      base = b;
      if (mode === 'export') path = `${b}\\${source}.tar`;
      if (mode === 'clone') {
        name = t('ops.cloneSuffix', { name: source });
        path = `${b}\\${name}`;
      }
    });
    const timer = setInterval(async () => {
      if (started) status = await invoke<Status>('op_status').catch(() => null);
    }, 1000);
    return () => clearInterval(timer);
  });

  async function browse(what: 'tar-open' | 'tar-save' | 'dir') {
    try {
      const filters = [{ name: t('ops.tarFilter'), extensions: ['tar'] }];
      if (what === 'tar-save') {
        const r = await save({ defaultPath: path, filters });
        if (r) path = r;
      } else if (what === 'tar-open') {
        const r = await open({ multiple: false, directory: false, filters });
        if (typeof r === 'string') tar = r;
      } else {
        const r = await open({ multiple: false, directory: true, defaultPath: base || undefined });
        if (typeof r === 'string') path = r;
      }
    } catch (e) {
      error = String(e);
    }
  }

  function suggestDir() {
    if (mode !== 'export' && base && (path === '' || path.startsWith(base))) path = `${base}\\${name}`;
  }

  async function go(arg?: boolean) {
    error = '';
    try {
      if (mode === 'convert') await invoke('op_convert', { name: source, version: version === 2 ? 1 : 2 });
      else if (mode === 'sparse') await invoke('op_sparse', { name: source, on: arg });
      else if (mode === 'wslupdate') await invoke('op_wsl_update');
      else if (mode === 'create') await invoke('op_create', { distro: selected, name: custom ? name : null, dir: custom ? path : null });
      else if (mode === 'export') await invoke('op_export', { name: source, tar: path });
      else if (mode === 'clone') await invoke('op_clone', { source, newName: name, dir: path });
      else await invoke('op_import', { name, dir: path, tar });
      started = true;
      status = { active: true, kind: mode, stage: t('ops.starting'), done: null, error: null, bytes: 0 };
    } catch (e) {
      error = String(e);
    }
  }
</script>

<Dialog title={title} {onclose} width={560}>
    {#if !started && mode === 'convert'}
      <p>
        {t('ops.convert.text', { from: version, to: version === 2 ? 1 : 2 })}
      </p>
      {#if error}<div class="error">{bm(error)}</div>{/if}
      <div class="foot">
        <button class="primary" onclick={() => go()}>{t('ops.convert.go', { to: version === 2 ? 1 : 2 })}</button>
        <button onclick={onclose}>{t('ops.cancel')}</button>
      </div>
    {:else if !started && mode === 'sparse'}
      <p>
        {t('ops.sparse.text')}
      </p>
      {#if sparseReason}<div class="error">{bm(sparseReason)}</div>{/if}
      {#if error}<div class="error">{bm(error)}</div>{/if}
      <div class="foot">
        <button class="primary" disabled={sparseReason !== ''} onclick={() => go(true)}>{t('ops.sparse.enable')}</button>
        <button disabled={sparseReason !== ''} onclick={() => go(false)}>{t('ops.sparse.disable')}</button>
        <button onclick={onclose}>{t('ops.cancel')}</button>
      </div>
    {:else if !started && mode === 'wslupdate'}
      {#if !upd}
        <p class="muted">{t('ops.update.searching')}</p>
      {:else}
        <p>{t('ops.update.installed')} <strong>{upd.current ?? t('ops.update.unknown')}</strong><br />
          {t('ops.update.latest')} <strong>{upd.latest ?? t('ops.update.unavailable')}</strong></p>
        {#if upd.latest && !upd.newer}<p class="muted">{t('ops.update.upToDate')}</p>{/if}
        <p class="muted">{t('ops.update.warning')}</p>
      {/if}
      {#if error}<div class="error">{bm(error)}</div>{/if}
      <div class="foot">
        <button class="primary" disabled={!upd} onclick={() => go()}>{t('ops.update.go')}</button>
        <button onclick={onclose}>{t('ops.cancel')}</button>
      </div>
    {:else if !started && mode === 'create'}
      {#if onlineLoading}
        <p class="muted">{t('ops.create.loading')}</p>
      {:else if online.length === 0}
        {#if error}<div class="error">{bm(error)}</div>{/if}
        <button onclick={loadOnline}>{t('ops.create.retry')}</button>
      {:else}
        <ul class="catalog" role="radiogroup" aria-label={t('ops.create.available')}>
          {#each online as d (d.name)}
            <li>
              <label class="opt"><input type="radio" name="distro" value={d.name} checked={selected === d.name} onchange={() => pickDistro(d.name)} />
                <span><strong>{d.friendly}</strong> <span class="muted">{d.name}</span></span>
              </label>
            </li>
          {/each}
        </ul>
        <p class="muted">{t('ops.create.weight')}</p>
        <label class="opt"><input type="checkbox" bind:checked={custom} disabled={customReason !== ''} onchange={toggleCustom} /> {t('ops.create.custom')}</label>
        {#if customReason}<p class="muted">{bm(customReason)}</p>{/if}
        {#if custom}
          <label>{t('ops.create.name')}
            <input bind:value={name} oninput={suggestDir} />
          </label>
          <label>{t('ops.create.dir')}
            <span class="pick"><input bind:value={path} /><button type="button" onclick={() => browse('dir')}>{t('ops.browse')}</button></span>
          </label>
        {/if}
        <p class="muted">{t('ops.create.default')}</p>
        {#if error}<div class="error">{bm(error)}</div>{/if}
        <div class="foot">
          <button class="primary" disabled={!selected} onclick={() => go()}>{t('ops.create.go')}</button>
          <button onclick={onclose}>{t('ops.cancel')}</button>
        </div>
      {/if}
    {:else if !started}
      {#if mode !== 'export'}
        <label>{t('ops.form.newName')}
          <input bind:value={name} oninput={suggestDir} placeholder={t('ops.form.namePlaceholder')} />
        </label>
      {/if}
      {#if mode === 'import'}
        <label>{t('ops.form.tarSource')}
          <span class="pick"><input bind:value={tar} placeholder="C:\sauvegardes\ubuntu.tar" /><button type="button" onclick={() => browse('tar-open')}>{t('ops.browse')}</button></span>
        </label>
      {/if}
      <label>{mode === 'export' ? t('ops.form.exportFile') : t('ops.form.installDir')}
        <span class="pick"><input bind:value={path} /><button type="button" onclick={() => browse(mode === 'export' ? 'tar-save' : 'dir')}>{t('ops.browse')}</button></span>
      </label>
      {#if mode === 'export'}
        <p class="muted">{t('ops.form.exportHint')}</p>
      {:else}
        <p class="muted">{t('ops.form.dirHint')} {mode === 'clone' ? t('ops.form.cloneHint') : ''}</p>
      {/if}
      {#if error}<div class="error">{bm(error)}</div>{/if}
      <div class="foot">
        <button class="primary" onclick={() => go()}>{mode === 'export' ? t('ops.form.export') : mode === 'clone' ? t('ops.form.clone') : t('ops.form.import')}</button>
        <button onclick={onclose}>{t('ops.cancel')}</button>
      </div>
    {:else if status}
      {#if status.active}
        <p>{bm(status.stage)}</p>
        <progress></progress>
        {#if status.bytes > 0}<p class="muted">{t('ops.progress.written', { size: size(status.bytes) })}</p>{/if}
        <button onclick={() => invoke('op_cancel')}>{t('ops.progress.cancel')}</button>
      {:else if status.error}
        <div class="error">{bm(status.error)}</div>
        <button onclick={onclose}>{t('ops.close')}</button>
      {:else}
        <div class="ok">{bm(status.done ?? "")}</div>
        <button onclick={onclose}>{t('ops.close')}</button>
      {/if}
    {/if}
</Dialog>

<style>
  label { display: block; margin-bottom: 0.7rem; font-size: 0.9rem; }
  input { display: block; width: 100%; box-sizing: border-box; margin-top: 0.2rem; padding: 0.4rem 0.6rem; border-radius: 6px; border: 1px solid var(--border); background: var(--surface-1); color: var(--text); }
  progress { width: 100%; }
  .catalog { list-style: none; margin: 0 0 0.6rem; padding: 0; max-height: 14rem; overflow: auto; border: 1px solid var(--border); border-radius: 6px; }
  .opt { display: flex; align-items: center; gap: 0.5rem; padding: 0.25rem 0.5rem; margin: 0; cursor: pointer; }
  .opt input { display: inline-block; width: auto; margin: 0; }
  .pick { display: flex; gap: 0.4rem; align-items: end; }
  .pick input { flex: 1; }
  .foot { display: flex; gap: 0.5rem; }
  .primary { background: #1f6feb; color: #fff; border-color: #1f6feb; }
  .muted { opacity: 0.65; font-size: 0.85rem; }
  .error { padding: 0.6rem 0.9rem; margin: 0.6rem 0; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  .ok { padding: 0.6rem 0.9rem; margin: 0.6rem 0; border-radius: 8px; background: #2ea04333; border: 1px solid #2ea043; }
  button:disabled { opacity: 0.5; cursor: default; }
  button { padding: 0.4rem 0.9rem; border-radius: 6px; border: 1px solid var(--border); background: var(--surface-1); color: var(--text); cursor: pointer; }
</style>
