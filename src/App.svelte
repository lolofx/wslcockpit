<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import OpsDialog from './lib/OpsDialog.svelte';
  import DeleteDialog from './lib/DeleteDialog.svelte';
  import ConfigPanel from './lib/ConfigPanel.svelte';
  import ProtoApp from './ProtoApp.svelte';
  import FeedbackHost from './ui/FeedbackHost.svelte';
  import { confirmAction, promptText, toast } from './ui/feedback.svelte';

  type Distro = { name: string; state: string; version: number; is_default: boolean; vhdx_bytes: number | null };

  let distros = $state<Distro[]>([]);
  let error = $state('');
  let loaded = $state(false);
  let aliases = $state<Record<string, string>>({});
  let policies = $state<Record<string, string>>({});
  type Mon = { ram_bytes: number; cpu_percent: number | null; disks: Record<string, [number, number]> };
  let mon = $state<Mon | null>(null);
  let update = $state<{ version: string; url: string } | null>(null);
  let autostart = $state(false);
  let busy = $state<string | null>(null);
  let proto = $state(false);
  let showConfig = $state(false);
  let deleting = $state<string | null>(null);
  let ops = $state<{ mode: 'export' | 'import' | 'clone' | 'convert' | 'sparse'; source: string; version?: number } | null>(null);

  async function toggleAutostart() {
    try {
      autostart = await invoke<boolean>('autostart_set', { on: !autostart });
    } catch (e) {
      error = String(e);
    }
  }

  async function pollMonitor() {
    if (document.hidden) return;
    try {
      mon = await invoke<Mon | null>('monitor');
    } catch {
      mon = null;
    }
  }

  async function setPolicy(name: string, policy: string) {
    try {
      policies = await invoke<Record<string, string>>('set_policy', { name, policy });
      if (policy === 'keepalive') await act('start', name);
    } catch (e) {
      error = String(e);
    }
  }

  const absent = $derived(error === 'WSL_ABSENT');

  async function rename(d: Distro) {
    const v = await promptText({
      title: `Alias pour « ${d.name} »`,
      message: "Vide = retirer l'alias. Affichage seulement, WSL n'est pas modifié.",
      value: aliases[d.name] ?? '',
    });
    if (v === null) return;
    try {
      aliases = await invoke<Record<string, string>>('set_alias', { name: d.name, alias: v });
    } catch (e) {
      error = String(e);
    }
  }

  const size = (b: number) =>
    b >= 1024 ** 3 ? `${(b / 1024 ** 3).toFixed(1)} Go` : `${Math.round(b / 1024 ** 2)} Mo`;

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

  async function makeDefault(name: string) {
    busy = name;
    try {
      await invoke('set_default_distro', { name });
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
    if (!(await confirmAction({ title: 'Tout arrêter ?', message: `wsl --shutdown\n\nEn marche : ${list}`, confirmLabel: 'Tout arrêter', danger: true }))) return;
    busy = '*';
    try {
      await invoke('shutdown');
      error = '';
      toast('Toutes les distributions sont arrêtées.', 'success');
    } catch (e) {
      error = String(e);
    }
    busy = null;
    await refresh();
  }

  onMount(() => {
    invoke<Record<string, string>>('aliases').then((a) => (aliases = a));
    invoke<Record<string, string>>('policies').then((p) => (policies = p));
    invoke('apply_policies').catch(() => {});
    invoke<boolean>('autostart_get').then((v) => (autostart = v));
    refresh();
    pollMonitor();
    invoke<{ version: string; url: string } | null>('check_update').then((u) => (update = u)).catch(() => {});
    const t = setInterval(() => !busy && refresh(), 3000);
    const m = setInterval(pollMonitor, 5000);
    return () => {
      clearInterval(t);
      clearInterval(m);
    };
  });
</script>

<FeedbackHost />
{#if proto}
  <ProtoApp onexit={() => (proto = false)} />
{:else}
<main>
  <header>
    <h1>WSLCockpit</h1>
    <button onclick={() => (proto = true)}>Prototype UX</button>
    <label class="auto"><input type="checkbox" checked={autostart} onchange={toggleAutostart} /> Démarrer avec Windows</label>
    <button onclick={() => (ops = { mode: 'import', source: '' })} style="margin-right: 0.5rem">Importer</button>
    <button onclick={() => (showConfig = true)} style="margin-right: 0.5rem">Configuration</button>
    <button onclick={shutdownAll} disabled={busy !== null}>Tout arrêter</button>
  </header>

  {#if showConfig}
    <ConfigPanel distros={distros.map((d) => d.name)} onclose={() => (showConfig = false)} />
  {:else}

  {#if update}
    <div class="update">
      Nouvelle version {update.version} disponible.
      <button class="link" onclick={() => invoke('open_url', { url: update!.url })}>Voir la release</button>
    </div>
  {/if}

  {#if mon}
    <div class="mon muted">
      RAM {size(mon.ram_bytes)}{#if mon.cpu_percent !== null} · CPU {mon.cpu_percent.toFixed(0)} %{/if}
    </div>
  {/if}

  {#if absent}
    <div class="error">
      <strong>WSL n'est pas installé ou pas activé.</strong><br />
      Ouvre PowerShell en administrateur et lance <code>wsl --install</code>, puis redémarre le PC.
    </div>
  {:else if error}
    <div class="error">{error}</div>
  {/if}

  {#if !loaded}
    <p class="muted">Chargement…</p>
  {:else if distros.length === 0 && !error && !absent}
    <p class="muted">Aucune distribution installée.</p>
  {/if}

  {#each distros as d (d.name)}
    <section class="card">
      <div class="info">
        <div class="title">
          <span class="dot" class:on={d.state === 'running'}></span>
          <strong>{aliases[d.name] ?? d.name}</strong>
          {#if aliases[d.name]}<span class="muted state">({d.name})</span>{/if}
          <button class="link" onclick={() => rename(d)} title="Renommer (alias local)">✎</button>
          <span class="tag">WSL{d.version}</span>
          {#if d.is_default}<span class="tag">défaut</span>{:else}<button class="link" onclick={() => makeDefault(d.name)} disabled={busy !== null} title="Définir comme distribution par défaut">Par défaut</button>{/if}
        </div>
        <div class="muted state meta">
          {label(d.state)}{#if mon?.disks[d.name]} · Disque {size(mon.disks[d.name][0])} / {size(mon.disks[d.name][1])}{/if}{#if d.vhdx_bytes !== null} · VHDX {size(d.vhdx_bytes)}{/if}
        </div>
      </div>
      <div class="actions">
        <select value={policies[d.name] ?? 'manual'} onchange={(e) => setPolicy(d.name, e.currentTarget.value)} title="Politique">
          <option value="manual">Manuel</option>
          <option value="start">Démarrer à l'ouverture</option>
          <option value="keepalive">Garder en vie</option>
        </select>
        <button onclick={() => (ops = { mode: 'export', source: d.name })} disabled={busy !== null}>Exporter</button>
        <button onclick={() => (ops = { mode: 'clone', source: d.name })} disabled={busy !== null}>Cloner</button>
        <button onclick={() => (deleting = d.name)} disabled={busy !== null || d.state === 'installing'}>Supprimer</button>
        {#if d.state === 'running'}
          <button onclick={() => act('terminal', d.name)} disabled={busy !== null}>Terminal</button>
          <button onclick={() => act('restart', d.name)} disabled={busy !== null}>Redémarrer</button>
          <button onclick={() => act('stop', d.name)} disabled={busy !== null}>Arrêter</button>
        {:else}
          <button onclick={() => (ops = { mode: 'convert', source: d.name, version: d.version })} disabled={busy !== null}>Convertir</button>
          <button onclick={() => (ops = { mode: 'sparse', source: d.name })} disabled={busy !== null}>Sparse</button>
          <button onclick={() => act('terminal', d.name)} disabled={busy !== null}>Terminal</button>
          <button onclick={() => act('start', d.name)} disabled={busy !== null}>Démarrer</button>
        {/if}
      </div>
    </section>
  {/each}
  {/if}
  {#if deleting}
    <DeleteDialog
      name={deleting}
      onclose={() => { deleting = null; refresh(); invoke<Record<string, string>>('aliases').then((a) => (aliases = a)); }}
      onexport={() => { ops = { mode: 'export', source: deleting! }; deleting = null; }}
    />
  {/if}
  {#if ops}
    <OpsDialog mode={ops.mode} source={ops.source} version={ops.version} onclose={() => { ops = null; refresh(); }} />
  {/if}
</main>
{/if}

<style>
  main { padding: 1.25rem; max-width: 760px; margin: 0 auto; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h1 { font-size: 1.4rem; margin: 0; }
  .card {
    display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.6rem;
    padding: 0.9rem 1rem; margin-bottom: 0.6rem; border-radius: 10px;
    background: var(--surface-1); border: 1px solid var(--border);
  }
  .info { flex: 1 1 260px; min-width: 0; }
  .title { display: flex; flex-wrap: wrap; align-items: center; gap: 0.4rem 0.6rem; }
  .meta { margin: 0.25rem 0 0 1.1rem; white-space: nowrap; }
  strong { overflow-wrap: anywhere; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: #888; }
  .dot.on { background: #2ea043; }
  .tag { font-size: 0.75rem; padding: 0.1rem 0.45rem; border-radius: 6px; background: color-mix(in srgb, CanvasText 12%, Canvas); }
  .muted { opacity: 0.65; }
  .actions { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .tag { white-space: nowrap; }
  .auto { font-size: 0.85rem; margin-left: auto; margin-right: 1rem; }
  .mon { margin-bottom: 0.8rem; font-size: 0.9rem; }
  .update { padding: 0.5rem 0.9rem; margin-bottom: 0.8rem; border-radius: 8px; background: #1f6feb33; border: 1px solid #1f6feb; font-size: 0.9rem; }
  .update .link { opacity: 1; text-decoration: underline; cursor: pointer; }
  .link { padding: 0 0.3rem; border: none; background: none; opacity: 0.6; }
  .state { font-size: 0.85rem; }
  .error { padding: 0.7rem 1rem; margin-bottom: 1rem; border-radius: 8px; background: #b4232333; border: 1px solid #b42323; white-space: pre-wrap; }
  button { font: inherit; font-size: var(--fs-sm); padding: var(--sp-2) var(--sp-3); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-2); }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
