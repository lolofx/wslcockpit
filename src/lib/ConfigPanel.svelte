<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';

  // `fixed` : distro imposée (bloc d'espace de travail) ; sans lui, onglets Global + distros.
  let { distros, onclose, fixed }: { distros: string[]; onclose?: () => void; fixed?: string } = $props();

  type FieldDef = { id: string; label: string; hint: string; options?: string[] };
  const GLOBAL: FieldDef[] = [
    { id: 'memory', label: 'Mémoire', hint: 'ex. 8GB (vide = défaut)' },
    { id: 'processors', label: 'Processeurs', hint: 'nombre entier ≥ 1' },
    { id: 'swap', label: 'Swap', hint: 'ex. 4GB, 0 = désactivé' },
    { id: 'networkingMode', label: 'Mode réseau', hint: '', options: ['', 'nat', 'mirrored', 'bridged', 'virtioproxy', 'none'] },
    { id: 'vmIdleTimeout', label: 'Arrêt de la VM inactive (ms)', hint: '-1 = jamais' }
  ];
  const DISTRO: FieldDef[] = [
    { id: 'systemd', label: 'systemd', hint: '', options: ['', 'true', 'false'] },
    { id: 'defaultUser', label: 'Utilisateur par défaut', hint: 'lettres, chiffres, _ - .' },
    { id: 'automountEnabled', label: 'Montage automatique des lecteurs', hint: '', options: ['', 'true', 'false'] }
  ];

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

  function pick(t: string | null) {
    target = t;
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
      baseMsg = 'Enregistré.'
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
      <h2>Configuration</h2>
      {#if onclose}<button onclick={onclose}>Retour</button>{/if}
    </div>
    <div class="tabs">
      <button class:active={target === null} onclick={() => pick(null)}>Global</button>
      {#each distros as d (d)}
        <button class:active={target === d} onclick={() => pick(d)}>{d}</button>
      {/each}
    </div>
  {/if}

  {#if target === null}
    <div class="row">
      <span>Dossier des nouvelles distributions</span>
      <div class="pick">
        <input bind:value={base} placeholder="C:\\WSL" onchange={saveBase} />
        <button onclick={browseBase}>Parcourir…</button>
      </div>
      <span class="muted">Utilisé par création, import et clonage : chaque distro va dans <code>dossier\nom</code>. Vide = <code>%USERPROFILE%\WSL</code>.</span>
      {#if baseMsg}<span class="muted">{baseMsg}</span>{/if}
    </div>
    <p class="muted">Fichier <code>.wslconfig</code> de ton profil Windows. Une valeur vide retire la clé.</p>
  {:else}
    <p class="muted">Fichier <code>/etc/wsl.conf</code> de la distribution. Une valeur vide retire la clé.</p>
  {/if}

  {#if needStart}
    <div class="warn">
      « {target} » est arrêtée. Pour lire son <code>wsl.conf</code>, elle doit démarrer.
      <button onclick={() => load(true)}>Démarrer et lire</button>
    </div>
  {:else if loaded}
    {#each fields as f (f.id)}
      <label class="row">
        <span>{f.label}</span>
        {#if f.options}
          <select bind:value={values[f.id]}>
            {#each f.options as o}<option value={o}>{o === '' ? '(non défini)' : o}</option>{/each}
          </select>
        {:else}
          <input bind:value={values[f.id]} placeholder={f.hint} class:bad={errors[f.id]} />
        {/if}
        {#if errors[f.id]}<span class="err">{errors[f.id]}</span>{/if}
      </label>
    {/each}
    <div class="foot">
      <button class="primary" onclick={save}>Enregistrer</button>
      <span class="muted">Une sauvegarde horodatée de l'ancien fichier est créée à côté.</span>
    </div>
    {#if saved}
      <div class="warn">
        Enregistré. Redémarrage requis : lance « Tout arrêter » (<code>wsl --shutdown</code>) puis relance les distributions pour appliquer.
      </div>
    {/if}
  {:else if !failure}
    <p class="muted">Chargement…</p>
  {/if}
  {#if failure}<div class="error">{failure}</div>{/if}
</section>

<style>
  .top { display: flex; justify-content: space-between; align-items: center; }
  h2 { font-size: 1.15rem; margin: 0; }
  .tabs { display: flex; flex-wrap: wrap; gap: 0.3rem; margin: 0.8rem 0; }
  .tabs .active { background: #1f6feb; color: #fff; border-color: #1f6feb; }
  .row { display: grid; grid-template-columns: 14rem 1fr; gap: 0.2rem 0.8rem; align-items: center; margin-bottom: 0.6rem; }
  .row input, .row select { padding: 0.4rem 0.6rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; }
  .pick { display: flex; gap: 0.4rem; }
  .pick input { flex: 1; min-width: 0; }
  .row > .muted { grid-column: 2; }
  .row .bad { border-color: #b42323; }
  .err { grid-column: 2; color: #e5534b; font-size: 0.85rem; }
  .foot { display: flex; align-items: center; gap: 0.8rem; margin-top: 1rem; }
  .primary { background: #1f6feb; color: #fff; border-color: #1f6feb; }
  .warn { padding: 0.6rem 0.9rem; margin: 0.8rem 0; border-radius: 8px; background: #d2992233; border: 1px solid #d29922; }
  .error { padding: 0.7rem 1rem; margin-top: 0.8rem; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  .muted { opacity: 0.65; font-size: 0.9rem; }
  button { padding: 0.4rem 0.9rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; cursor: pointer; }
</style>
