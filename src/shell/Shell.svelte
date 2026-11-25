<script lang="ts">
  import { t, bm } from '../i18n/index.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { app, nav, display, find, act, size, stateLabel, toggleAutostart, shutdownAll, refresh, reloadAliases } from '../state/app.svelte'
  import { buildCommands } from './commands'
  import { distroMenu, viewMenu, newView, type MenuItem } from './menus'
  import { layouts, findView } from '../state/layouts.svelte'
  import { viewKey } from '../workspace/model'
  import Palette from './Palette.svelte'
  import Notifications from './Notifications.svelte'
  import Overview from './Overview.svelte'
  import Workspace from '../workspace/Workspace.svelte'
  import ConfigPanel from '../lib/ConfigPanel.svelte'
  import OpsDialog from '../lib/OpsDialog.svelte'
  import DeleteDialog from '../lib/DeleteDialog.svelte'
  import Button from '../ui/Button.svelte'
  import LaunchersPanel from '../lib/LaunchersPanel.svelte'
  import { flip } from 'svelte/animate'
  import { scale } from 'svelte/transition'
  import Icon from '../icons/Icon.svelte'
  import { distroIcon } from '../state/icons.svelte'
  import Menu from '../ui/Menu.svelte'
  import Badge from '../ui/Badge.svelte'

  let view = $state('overview') // 'overview' | 'settings' | nom d'une distro
  let drawer = $state(false)
  let palette = $state(false)
  let bell = $state(false)

  const go = (v: string) => { view = v; drawer = false }
  nav.go = go
  const commands = $derived(buildCommands({ go, current: () => view }))
  const current = $derived(find(view))
  const curView = $derived(view.startsWith('view:') ? findView(view.slice(5)) : undefined)
  const absent = $derived(app.error === 'WSL_ABSENT')
  const running = $derived(app.distros.filter((d) => d.state === 'running').length)

  // Une distro supprimée ne laisse pas un écran orphelin.
  $effect(() => {
    if (app.loaded && layouts.loaded && view !== 'overview' && view !== 'settings' && !current && !curView) view = 'overview'
  })

  function key(e: KeyboardEvent) {
    // Dans un terminal, Ctrl+K appartient au shell (effacer jusqu'en fin de ligne).
    if ((e.target as HTMLElement | null)?.closest?.('.xterm')) return
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); palette = !palette }
  }

  const globalMenu = $derived<MenuItem[]>([
    { label: t('shell.global.create'), onselect: () => (app.ops = { mode: 'create', source: '' }) },
    { label: t('shell.global.import'), onselect: () => (app.ops = { mode: 'import', source: '' }) },
    { label: t('shell.global.wslUpdate'), onselect: () => (app.ops = { mode: 'wslupdate', source: '' }) },
    { label: t('shell.global.settings'), onselect: () => go('settings') },
    { label: app.autostart ? t('shell.global.autostartOn') : t('shell.global.autostartOff'), onselect: () => void toggleAutostart() },
    { label: t('shell.global.shutdownAll'), onselect: () => void shutdownAll(), danger: true, disabled: app.busy !== null },
  ])
</script>

<svelte:window onkeydown={key} />

<div class="shell">
  <nav class="rail" class:open={drawer} aria-label={t('shell.nav.label')}>
    <button class="item" class:on={view === 'overview'} title={t('shell.nav.overview')} onclick={() => go('overview')}>
      <span class="ic">▦</span><span class="lb">{t('shell.nav.overview')}</span>
    </button>
    <div class="sep"></div>
    <div class="list">
      {#each app.distros as d (d.name)}
        <button class="item" animate:flip={{ duration: 150 }} in:scale={{ duration: 150, start: 0.7 }} class:on={view === d.name} title={display(d.name)} onclick={() => go(d.name)}>
          <span class="ic"><Icon spec={distroIcon(d.name)} label={display(d.name)} size={26} /><i class="dot" class:run={d.state === 'running'}></i></span>
          <span class="lb">{display(d.name)}</span>
        </button>
      {/each}
    </div>
    <div class="sep"></div>
    <div class="list views">
      {#each layouts.doc.views as v (v.id)}
        <button class="item" animate:flip={{ duration: 150 }} in:scale={{ duration: 150, start: 0.7 }} class:on={view === viewKey(v.id)} title={v.name} onclick={() => go(viewKey(v.id))}>
          <span class="ic"><Icon spec={v.icon} label={v.name} size={26} /></span>
          <span class="lb">{v.name}</span>
        </button>
      {/each}
      <button class="item" title={t('shell.nav.newView')} aria-label={t('shell.nav.newView')} onclick={() => void newView(go)}>
        <span class="ic plus">+</span><span class="lb">{t('shell.nav.newView')}</span>
      </button>
    </div>
    <button class="item" class:on={view === 'settings'} title={t('shell.nav.settings')} onclick={() => go('settings')}>
      <span class="ic">⚙</span><span class="lb">{t('shell.nav.settings')}</span>
    </button>
  </nav>
  {#if drawer}<button class="scrim" aria-label={t('shell.nav.closeMenu')} onclick={() => (drawer = false)}></button>{/if}

  <div class="main">
    <header class="top">
      <button class="hamb" aria-label={t('shell.nav.menu')} onclick={() => (drawer = !drawer)}>☰</button>
      {#if current}
        <h1>{display(current.name)}</h1>
        <Badge tone={current.state === 'running' ? 'success' : 'neutral'}>{stateLabel(current.state)}</Badge>
        <div class="grow"></div>
        {#if current.state === 'running'}
          <Button onclick={() => act('stop', current.name)} disabled={app.busy !== null}>{t('shell.top.stop')}</Button>
        {:else}
          <Button variant="primary" onclick={() => act('start', current.name)} disabled={app.busy !== null}>{t('shell.top.start')}</Button>
        {/if}
        <Menu items={distroMenu(current)} label={t('shell.top.actionsOf', { name: display(current.name) })} />
      {:else if curView}
        <Icon spec={curView.icon} label={curView.name} size={24} />
        <h1>{curView.name}</h1>
        <div class="grow"></div>
        <Menu items={viewMenu(curView.id, go)} label={t('shell.top.viewActions')} />
      {:else}
        <h1>{view === 'settings' ? t('shell.nav.settings') : t('shell.nav.overview')}</h1>
        <div class="grow"></div>
        <Menu items={globalMenu} label={t('shell.top.globalActions')} />
      {/if}
      <Button variant="ghost" onclick={() => { bell = true; app.unread = 0 }} title={t('shell.top.notifications')} aria-label={t('shell.top.notifications')}>🔔{#if app.unread > 0}<span class="n">{app.unread > 99 ? '99+' : app.unread}</span>{/if}</Button>
      <Button variant="ghost" onclick={() => (palette = true)} title={t('shell.top.palette')}>Ctrl K</Button>
    </header>

    {#if app.update}
      <div class="banner info">
        {t('shell.banner.update', { version: app.update.version })}
        <button class="lnk" onclick={() => invoke('open_url', { url: app.update!.url })}>{t('shell.banner.release')}</button>
      </div>
    {/if}
    {#if absent}
      <div class="banner err">
        <strong>{t('shell.banner.wslAbsentTitle')}</strong> {t('shell.banner.wslAbsentBefore')} <code>wsl --install</code>{t('shell.banner.wslAbsentAfter')}
      </div>
    {:else if app.error}
      <div class="banner err">{bm(app.error)}</div>
    {/if}

    <section class="content">
      {#if view === 'overview'}
        <div class="scroll"><Overview open={go} /></div>
      {:else if view === 'settings'}
        <div class="scroll"><ConfigPanel distros={app.distros.map((d) => d.name)} onclose={() => go('overview')} /><LaunchersPanel /></div>
      {:else if current}
        <Workspace distro={current.name} />
      {:else if curView}
        <Workspace distro="" space={viewKey(curView.id)} projectRoot={curView.root} views />
      {/if}
    </section>

    <footer class="status">
      <span>{t('shell.status.running', { running, total: app.distros.length })}</span>
      {#if app.mon}<span>RAM {size(app.mon.ram_bytes)}{#if app.mon.cpu_percent !== null} · CPU {app.mon.cpu_percent.toFixed(0)} %{/if}</span>{/if}
      {#if app.opStage}<span>{app.opStage}</span>{:else if app.busy}<span>{t('shell.status.working')}</span>{/if}
    </footer>
  </div>
</div>

{#if bell}<Notifications onclose={() => (bell = false)} />{/if}
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
  .rail { flex: none; width: 3.6rem; display: flex; flex-direction: column; gap: var(--sp-1); padding: var(--sp-2); margin: 2px 0 2px 2px; background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
  .list { flex: 1; display: flex; flex-direction: column; gap: var(--sp-1); overflow-y: auto; overflow-x: hidden; }
  .list.views { flex: none; max-height: 40%; }
  .plus { font-size: 1.3rem; color: var(--text-dim); }
  .sep { height: 1px; background: var(--border); margin: var(--sp-1) 0; }
  .item { display: flex; align-items: center; gap: var(--sp-3); border: 1px solid transparent; border-radius: var(--radius); background: none; color: var(--text); padding: var(--sp-1); cursor: pointer; font: inherit; text-align: left; transition: background var(--dur), border-color var(--dur), color var(--dur); }
  .item:hover { background: var(--surface-3); }
  .item:focus-visible { outline: none; box-shadow: var(--focus); }
  .item.on { background: var(--tint); border-color: var(--accent); color: var(--accent); }
  .ic { position: relative; flex: none; width: 2.2rem; height: 2.2rem; display: grid; place-items: center; font-size: var(--fs-sm); font-weight: 600; }
  .dot { transition: background var(--dur); position: absolute; right: 2px; bottom: 2px; width: 8px; height: 8px; border-radius: 50%; background: var(--text-dim); border: 2px solid var(--surface-2); box-sizing: content-box; }
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
  .n { margin-left: 2px; padding: 0 5px; border-radius: 999px; background: var(--danger); color: var(--on-danger); font-size: 0.7rem; }

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
