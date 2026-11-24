<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { mode, source, onclose }: { mode: 'export' | 'import' | 'clone'; source: string; onclose: () => void } = $props();

  type Status = { active: boolean; kind: string; stage: string; done: string | null; error: string | null; bytes: number };

  let base = $state('');
  let name = $state('');
  let path = $state(''); // export : archive .tar ; import/clone : dossier d'installation
  let tar = $state(''); // import : archive source
  let status = $state<Status | null>(null);
  let error = $state('');
  let started = $state(false);

  const size = (b: number) =>
    b >= 1024 ** 3 ? `${(b / 1024 ** 3).toFixed(2)} Go` : `${Math.round(b / 1024 ** 2)} Mo`;

  const title = $derived(
    mode === 'export' ? `Exporter « ${source} »` : mode === 'clone' ? `Cloner « ${source} »` : 'Importer une distribution'
  );

  onMount(() => {
    invoke<string>('op_default_base').then((b) => {
      base = b;
      if (mode === 'export') path = `${b}\\${source}.tar`;
      if (mode === 'clone') {
        name = `${source}-copie`;
        path = `${b}\\${name}`;
      }
    });
    const t = setInterval(async () => {
      if (started) status = await invoke<Status>('op_status').catch(() => null);
    }, 1000);
    return () => clearInterval(t);
  });

  function suggestDir() {
    if (mode !== 'export' && base && (path === '' || path.startsWith(base))) path = `${base}\\${name}`;
  }

  async function go() {
    error = '';
    try {
      if (mode === 'export') await invoke('op_export', { name: source, tar: path });
      else if (mode === 'clone') await invoke('op_clone', { source, newName: name, dir: path });
      else await invoke('op_import', { name, dir: path, tar });
      started = true;
      status = { active: true, kind: mode, stage: 'Démarrage…', done: null, error: null, bytes: 0 };
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="overlay">
  <div class="dialog">
    <h2>{title}</h2>
    {#if !started}
      {#if mode !== 'export'}
        <label>Nom de la nouvelle distribution
          <input bind:value={name} oninput={suggestDir} placeholder="ex. Ubuntu-test" />
        </label>
      {/if}
      {#if mode === 'import'}
        <label>Archive à importer (.tar)
          <input bind:value={tar} placeholder="C:\sauvegardes\ubuntu.tar" />
        </label>
      {/if}
      <label>{mode === 'export' ? 'Fichier d\'export (.tar)' : 'Dossier d\'installation (vide ou à créer)'}
        <input bind:value={path} />
      </label>
      {#if mode === 'export'}
        <p class="muted">Un fichier existant n'est jamais écrasé. L'export peut prendre plusieurs minutes.</p>
      {:else}
        <p class="muted">Le dossier ne doit pas contenir de fichiers. {mode === 'clone' ? 'Le clone passe par une archive temporaire, supprimée ensuite.' : ''}</p>
      {/if}
      {#if error}<div class="error">{error}</div>{/if}
      <div class="foot">
        <button class="primary" onclick={go}>{mode === 'export' ? 'Exporter' : mode === 'clone' ? 'Cloner' : 'Importer'}</button>
        <button onclick={onclose}>Annuler</button>
      </div>
    {:else if status}
      {#if status.active}
        <p>{status.stage}</p>
        <progress></progress>
        <p class="muted">{size(status.bytes)} écrits</p>
        <button onclick={() => invoke('op_cancel')}>Annuler l'opération</button>
      {:else if status.error}
        <div class="error">{status.error}</div>
        <button onclick={onclose}>Fermer</button>
      {:else}
        <div class="ok">{status.done}</div>
        <button onclick={onclose}>Fermer</button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: #0008; display: flex; align-items: center; justify-content: center; padding: 1rem; }
  .dialog { background: Canvas; color: CanvasText; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); border-radius: 10px; padding: 1.2rem; width: min(560px, 100%); }
  h2 { font-size: 1.1rem; margin: 0 0 0.8rem; overflow-wrap: anywhere; }
  label { display: block; margin-bottom: 0.7rem; font-size: 0.9rem; }
  input { display: block; width: 100%; box-sizing: border-box; margin-top: 0.2rem; padding: 0.4rem 0.6rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; }
  progress { width: 100%; }
  .foot { display: flex; gap: 0.5rem; }
  .primary { background: #1f6feb; color: #fff; border-color: #1f6feb; }
  .muted { opacity: 0.65; font-size: 0.85rem; }
  .error { padding: 0.6rem 0.9rem; margin: 0.6rem 0; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  .ok { padding: 0.6rem 0.9rem; margin: 0.6rem 0; border-radius: 8px; background: #2ea04333; border: 1px solid #2ea043; }
  button { padding: 0.4rem 0.9rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; cursor: pointer; }
</style>
