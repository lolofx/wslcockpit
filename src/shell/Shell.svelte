<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { app, display, find, act, size, stateLabel, toggleAutostart, shutdownAll, refresh, reloadAliases } from '../state/app.svelte'
  import { buildCommands } from './commands'
  import { distroMenu, type MenuItem } from './menus'
  import Palette from './Palette.svelte'
  import Overview from './Overview.svelte'
  import Workspace from '../workspace/Workspace.svelte'
  import ConfigPanel from '../lib/ConfigPanel.svelte'
  import OpsDialog from '../lib/OpsDialog.svelte'
  import DeleteDialog from '../lib/DeleteDialog.svelte'
  import Button from '../ui/Button.svelte'
  import Menu from '../ui/Menu.svelte'
  import Badge from '../ui/Badge.svelte'

  let view = $state('overview') // 'overview' | 'settings' | nom d'une distro
  let drawer = $state(false)
  let palette = $state(false)

  const go = (v: string) => { view = v; drawer = false }
  const commands = $derived(buildCommands({ go, current: () => view }))
  const current = $derived(find(view))
  const absent = $derived(app.error === 'WSL_ABSENT')
  const running = $derived(app.distros.filter((d) => d.state === 'running').length)

  // Une distro supprimée ne laisse pas un écran orphelin.
  $effect(() => {
    if (app.loaded && view !== 'overview' && view !== 'settings' && !current) view = 'overview'
  })

  function key(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); palette = !palette }
  }

  const globalMenu = $derived<MenuItem[]>([
    { label: 'Importer une distribution…', onselect: () => (app.ops = { mode: 'import', source: '' }) },
    { label: 'Configuration…', onselect: () => go('settings') },
    { label: `Démarrer avec Windows : ${app.autostart ? 'oui' : 'non'}`, onselect: () => void toggleAutostart() },
    { label: 'Tout arrêter…', onselect: () => void shutdownAll(), danger: true, disabled: app.busy !== null },
  ])
</script>

<svelte:window onkeydown={key} />

<div class="shell">
  <nav class="rail" class:open={drawer} aria-label="Navigation">
    <button class="item" class:on={view === 'overview'} title="Vue d’ensemble" onclick={() => go('overview')}>
      <span class="ic">▦</span><span class="lb">Vue d’ensemble</span>
    </button>
    <div class="sep"></div>
    <div class="list">
      {#each app.distros as d (d.name)}
        <button class="item" class:on={view === d.name} title={display(d.name)} onclick={() => go(d.name)}>
          <span class="ic">{display(d.name).slice(0, 2).toUpperCase()}<i class="dot" class:run={d.state === 'running'}></i></span>
          <span class="lb">{display(d.name)}</span>
        </button>
      {/each}
    </div>
    <button class="item" class:on={view === 'settings'} title="Configuration" onclick={() => go('settings')}>
      <span class="ic">⚙</span><span class="lb">Configuration</span>
    </button>
  </nav>
  {#if drawer}<button class="scrim" aria-label="Fermer le menu" onclick={() => (drawer = false)}></button>{/if}

  <div class="main">
    <header class="top">
      <button class="hamb" aria-label="Menu" onclick={() => (drawer = !drawer)}>☰</button>
      {#if current}
        <h1>{display(current.name)}</h1>
        <Badge tone={current.state === 'running' ? 'success' : 'neutral'}>{stateLabel(current.state)}</Badge>
        <div class="grow"></div>
        {#if current.state === 'running'}
          <Button onclick={() => act('stop', current.name)} disabled={app.busy !== null}>Arrêter</Button>
        {:else}
          <Button variant="primary" onclick={() => act('start', current.name)} disabled={app.busy !== null}>Démarrer</Button>
        {/if}
        <Menu items={distroMenu(current)} label="Actions de {display(current.name)}" />
      {:else}
        <h1>{view === 'settings' ? 'Configuration' : 'Vue d’ensemble'}</h1>
        <div class="grow"></div>
        <Menu items={globalMenu} label="Actions générales" />
      {/if}
      <Button variant="ghost" onclick={() => (palette = true)} title="Palette de commandes (Ctrl+K)">Ctrl K</Button>
    </header>

    {#if app.update}
      <div class="banner info">
        Nouvelle version {app.update.version} disponible.
        <button class="lnk" onclick={() => invoke('open_url', { url: app.update!.url })}>Voir la release</button>
      </div>
    {/if}
    {#if absent}
      <div class="banner err">
        <strong>WSL n'est pas installé ou pas activé.</strong> Ouvre PowerShell en administrateur et lance <code>wsl --install</code>, puis redémarre le PC.
      </div>
    {:else if app.error}
      <div class="banner err">{app.error}</div>
    {/if}

    <section class="content">
      {#if view === 'overview'}
        <div class="scroll"><Overview open={go} /></div>
      {:else if view === 'settings'}
        <div class="scroll"><ConfigPanel distros={app.distros.map((d) => d.name)} onclose={() => go('overview')} /></div>
      {:else if current}
        <Workspace distro={current.name} />
      {/if}
    </section>

    <footer class="status">
      <span>{running}/{app.distros.length} en marche</span>
      {#if app.mon}<span>RAM {size(app.mon.ram_bytes)}{#if app.mon.cpu_percent !== null} · CPU {app.mon.cpu_percent.toFixed(0)} %{/if}</span>{/if}
      {#if app.busy}<span>Opération en cours…</span>{/if}
    </footer>
  </div>
</div>

{#if palette}<Palette {commands} onclose={() => (palette = false)} />{/if}

{#if app.deleting}
  <DeleteDialog
    name={app.deleting}
    onclose={() => { app.deleting = null; refresh(); reloadAliases() }}
    onexport={() => { app.ops = { mode: 'export', source: app.deleting! }; app.deleting = null }}
  />
{/if}
{#if app.ops}
  <OpsDialog mode={app.ops.mode} source={app.ops.source} version={app.ops.version} onclose={() => { app.ops = null; refresh() }} />
{/if}

<style>
  .shell { display: flex; height: 100vh; min-width: 0; }
  .rail { flex: none; width: 3.6rem; display: flex; flex-direction: column; gap: var(--sp-1); padding: var(--sp-2); background: var(--surface-1); border-right: 1px solid var(--border); overflow: hidden; }
  .list { flex: 1; display: flex; flex-direction: column; gap: var(--sp-1); overflow-y: auto; overflow-x: hidden; }
  .sep { height: 1px; background: var(--border); margin: var(--sp-1) 0; }
  .item { display: flex; align-items: center; gap: var(--sp-3); border: 1px solid transparent; border-radius: var(--radius); background: none; color: var(--text); padding: var(--sp-1); cursor: pointer; font: inherit; text-align: left; }
  .item:hover { background: var(--surface-2); }
  .item:focus-visible { outline: none; box-shadow: var(--focus); }
  .item.on { background: var(--surface-3); border-color: var(--accent); }
  .ic { position: relative; flex: none; width: 2.2rem; height: 2.2rem; display: grid; place-items: center; font-size: var(--fs-sm); font-weight: 600; }
  .dot { position: absolute; right: 2px; bottom: 2px; width: 8px; height: 8px; border-radius: 50%; background: var(--text-dim); border: 2px solid var(--surface-1); box-sizing: content-box; }
  .dot.run { background: var(--success); }
  .lb { display: none; font-size: var(--fs-sm); }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .top { display: flex; align-items: center; gap: var(--sp-2); padding: var(--sp-2) var(--sp-4); border-bottom: 1px solid var(--border); background: var(--surface-1); }
  h1 { margin: 0; font-size: var(--fs-lg); overflow-wrap: anywhere; }
  .grow { flex: 1; }
  .hamb { display: none; border: 0; background: none; color: var(--text); font-size: 1.2rem; cursor: pointer; }
  .banner { padding: var(--sp-2) var(--sp-4); font-size: var(--fs-sm); border-bottom: 1px solid var(--border); white-space: pre-wrap; }
  .banner.info { background: color-mix(in srgb, var(--accent) 14%, transparent); }
  .banner.err { background: color-mix(in srgb, var(--danger) 14%, transparent); }
  .lnk { border: 0; background: none; color: var(--accent); text-decoration: underline; cursor: pointer; font: inherit; }
  .content { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .scroll { flex: 1; overflow: auto; padding: var(--sp-4); }
  .status { display: flex; gap: var(--sp-4); padding: var(--sp-1) var(--sp-4); border-top: 1px solid var(--border); background: var(--surface-1); font-size: var(--fs-sm); color: var(--text-dim); }
  .scrim { display: none; }

  @media (max-width: 700px) {
    .shell { height: 100dvh; }
    .rail { position: fixed; z-index: 60; inset: 0 auto 0 0; width: 14rem; transform: translateX(-100%); transition: transform var(--dur); }
    .rail.open { transform: none; box-shadow: var(--shadow); }
    .lb { display: inline; }
    .hamb { display: block; }
    .scrim { display: block; position: fixed; inset: 0; z-index: 55; border: 0; background: var(--overlay); }
    .top { padding: var(--sp-2); flex-wrap: wrap; }
  }
</style>
