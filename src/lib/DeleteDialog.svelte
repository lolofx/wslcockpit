<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let { name, onclose, onexport }: { name: string; onclose: () => void; onexport: () => void } = $props();

  let typed = $state('');
  let error = $state('');
  let busy = $state(false);

  async function remove() {
    busy = true;
    error = '';
    try {
      await invoke('delete_distro', { name, typed });
      onclose();
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }
</script>

<div class="overlay">
  <div class="dialog">
    <h2>Supprimer « {name} »</h2>
    <p>
      Irréversible : la distribution, tous ses fichiers et son disque virtuel seront effacés.
      Pense à l'exporter d'abord.
    </p>
    <label>Tape le nom exact pour confirmer
      <input bind:value={typed} placeholder={name} />
    </label>
    {#if error}<div class="error">{error}</div>{/if}
    <div class="foot">
      <button onclick={onexport} disabled={busy}>Exporter d'abord</button>
      <button class="danger" onclick={remove} disabled={busy || typed !== name}>Supprimer</button>
      <button onclick={onclose} disabled={busy}>Annuler</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: #0008; display: flex; align-items: center; justify-content: center; padding: 1rem; }
  .dialog { background: Canvas; color: CanvasText; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); border-radius: 10px; padding: 1.2rem; width: min(520px, 100%); }
  h2 { font-size: 1.1rem; margin: 0 0 0.8rem; overflow-wrap: anywhere; }
  label { display: block; margin-bottom: 0.7rem; font-size: 0.9rem; }
  input { display: block; width: 100%; box-sizing: border-box; margin-top: 0.2rem; padding: 0.4rem 0.6rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; }
  .foot { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .danger { background: #b42323; color: #fff; border-color: #b42323; }
  .error { padding: 0.6rem 0.9rem; margin: 0.6rem 0; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  button { padding: 0.4rem 0.9rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; cursor: pointer; }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
