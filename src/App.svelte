<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  type Distro = { name: string; state: string; version: number; is_default: boolean };

  let distros = $state<Distro[]>([]);
  let error = $state('');
  let loaded = $state(false);
  let busy = $state<string | null>(null);

  const label = (s: string) =>
    s === 'running' ? 'En marche' : s === 'stopped' ? 'Arrêtée' : s === 'installing' ? 'Installation' : s;

  async function refresh() {
    try {
      distros = await invoke<Distro[]>('distros');
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      loaded = true;
    }
  }

  async function act(cmd: 'start' | 'stop' | 'restart' | 'terminal', name: string) {
    busy = name;
    try {
      await invoke(cmd, { name });
      if (cmd === 'start' || cmd === 'restart') await new Promise((r) => setTimeout(r, 1500));
      error = '';
    } catch (e) {
      error = String(e);
    }
    busy = null;
    await refresh();
  }

  async function shutdownAll() {
    const running = distros.filter((d) => d.state === 'running').map((d) => d.name);
    const list = running.length ? running.join(', ') : 'aucune distribution en marche';
    if (!confirm(`Tout arrêter (wsl --shutdown) ?\n\nEn marche : ${list}`)) return;
    busy = '*';
    try {
      await invoke('shutdown');
      error = '';
    } catch (e) {
      error = String(e);
    }
    busy = null;
    await refresh();
  }

  onMount(() => {
    refresh();
    const t = setInterval(() => !busy && refresh(), 3000);
    return () => clearInterval(t);
  });
</script>

<main>
  <header>
    <h1>WSLCockpit</h1>
    <button onclick={shutdownAll} disabled={busy !== null}>Tout arrêter</button>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if !loaded}
    <p class="muted">Chargement…</p>
  {:else if distros.length === 0 && !error}
    <p class="muted">Aucune distribution installée.</p>
  {/if}

  {#each distros as d (d.name)}
    <section class="card">
      <div class="title">
        <span class="dot" class:on={d.state === 'running'}></span>
        <strong>{d.name}</strong>
        <span class="tag">WSL{d.version}</span>
        {#if d.is_default}<span class="tag">défaut</span>{/if}
        <span class="muted state">{label(d.state)}</span>
      </div>
      <div class="actions">
        {#if d.state === 'running'}
          <button onclick={() => act('terminal', d.name)} disabled={busy !== null}>Terminal</button>
          <button onclick={() => act('restart', d.name)} disabled={busy !== null}>Redémarrer</button>
          <button onclick={() => act('stop', d.name)} disabled={busy !== null}>Arrêter</button>
        {:else}
          <button onclick={() => act('terminal', d.name)} disabled={busy !== null}>Terminal</button>
          <button onclick={() => act('start', d.name)} disabled={busy !== null}>Démarrer</button>
        {/if}
      </div>
    </section>
  {/each}
</main>

<style>
  main { padding: 1.25rem; max-width: 760px; margin: 0 auto; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h1 { font-size: 1.4rem; margin: 0; }
  .card {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.9rem 1rem; margin-bottom: 0.6rem; border-radius: 10px;
    background: color-mix(in srgb, CanvasText 6%, Canvas); border: 1px solid color-mix(in srgb, CanvasText 14%, Canvas);
  }
  .title { display: flex; align-items: center; gap: 0.6rem; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: #888; }
  .dot.on { background: #2ea043; }
  .tag { font-size: 0.75rem; padding: 0.1rem 0.45rem; border-radius: 6px; background: color-mix(in srgb, CanvasText 12%, Canvas); }
  .muted { opacity: 0.65; }
  .actions { display: flex; gap: 0.4rem; }
  .state { font-size: 0.85rem; }
  .error { padding: 0.7rem 1rem; margin-bottom: 1rem; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  button { padding: 0.4rem 0.9rem; border-radius: 6px; border: 1px solid color-mix(in srgb, CanvasText 25%, Canvas); background: Canvas; color: CanvasText; cursor: pointer; }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
