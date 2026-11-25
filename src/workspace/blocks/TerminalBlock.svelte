<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import '@xterm/xterm/css/xterm.css'
  import type { BlockContext } from '../registry'
  import { terminalSpec, parseOsc7, parseOsc9, type TerminalProps } from '../terminals'
  import Button from '../../ui/Button.svelte'
  import { t } from '../../i18n/index.svelte'

  let { distro, block, update }: BlockContext = $props()
  const id = block.id // la session vit côté Rust sous cet identifiant, indépendamment de l'affichage
  const p = $derived(block.props as unknown as TerminalProps)
  const spec = $derived(terminalSpec(distro, p, t('workspace.title.profileSuffix')))
  const mode = $derived(p.kind === 'wsl' ? 'wsl' : p.start === 'profile' ? 'profile' : 'distro')

  let host: HTMLDivElement
  let term: Terminal
  let fit: FitAddon
  let exited = $state(false)
  let failure = $state('')
  let settings = $state(false)
  let replayEnd = -1 // -1 : rejeu pas encore reçu, les sorties directes sont mises de côté
  let pending: { data: string; end: number }[] = []

  const bytes = (b64: string) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
  const css = (n: string, fallback: string) => getComputedStyle(document.documentElement).getPropertyValue(n).trim() || fallback
  const theme = () => ({
    background: css('--surface-1', '#1e1e1e'),
    foreground: css('--text', '#e6e6e6'),
    cursor: css('--accent', '#3b82f6'),
    selectionBackground: css('--surface-3', '#44475a'),
  })

  function direct(m: { data: string; end: number }) {
    if (m.end <= replayEnd) return // déjà inclus dans le rejeu
    term.write(bytes(m.data))
  }

  async function start() {
    failure = ''
    exited = false
    replayEnd = -1
    pending = []
    try {
      fitNow()
      const o = await invoke<{ replay: string; end: number; alive: boolean }>('pty_open', {
        id, program: spec.program, args: spec.args, cwd: spec.cwd, cols: term.cols, rows: term.rows, env: spec.env,
      })
      if (o.replay) term.write(bytes(o.replay))
      replayEnd = o.end
      pending.forEach(direct)
      pending = []
      exited = !o.alive
    } catch (e) {
      failure = String(e)
      replayEnd = 0
    }
  }

  async function restart() {
    await invoke('pty_close', { id })
    term.reset()
    await start()
  }

  function fitNow() {
    if (host && host.clientWidth > 0 && host.clientHeight > 0) {
      try { fit.fit() } catch { /* hôte masqué */ }
    }
  }

  onMount(() => {
    term = new Terminal({ fontFamily: "'Cascadia Mono', Consolas, monospace", fontSize: 13, cursorBlink: true, scrollback: 5000, theme: theme() })
    fit = new FitAddon()
    term.loadAddon(fit)
    // URL affichées par un outil (ex. aspire run) : Ctrl+clic ouvre le navigateur du système.
    term.loadAddon(new WebLinksAddon((e, url) => { if (e.ctrlKey || e.metaKey) void invoke('open_external', { url }).catch((x) => (failure = String(x))) }))
    term.open(host)
    // Dossier courant annoncé par le shell : retenu dans la disposition pour la prochaine ouverture de l'appli.
    const remember = (dir: string | null) => { if (dir && dir !== (block.props as { last?: string }).last) update({ last: dir }) }
    term.parser.registerOscHandler(7, (d) => { remember(parseOsc7(d)); return true })
    term.parser.registerOscHandler(9, (d) => { remember(parseOsc9(d)); return true })
    // Ctrl+C copie quand il y a une sélection (sinon c'est l'interruption du shell) ; Ctrl+Shift+C/V toujours.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== 'keydown' || !e.ctrlKey) return true
      const k = e.key.toLowerCase()
      if ((k === 'c' && (term.hasSelection() || e.shiftKey))) {
        void navigator.clipboard.writeText(term.getSelection())
        return false
      }
      if (k === 'v' && e.shiftKey) {
        void navigator.clipboard.readText().then((txt) => term.paste(txt))
        return false
      }
      return true
    })
    term.onData((d) => void invoke('pty_write', { id, data: d }).catch(() => {}))
    term.onResize(({ cols, rows }) => void invoke('pty_resize', { id, cols, rows }).catch(() => {}))

    let raf = 0
    const ro = new ResizeObserver(() => {
      cancelAnimationFrame(raf)
      raf = requestAnimationFrame(fitNow)
    })
    ro.observe(host)
    const mq = matchMedia('(prefers-color-scheme: dark)')
    const onScheme = () => (term.options.theme = theme())
    mq.addEventListener('change', onScheme)

    const un: Promise<() => void>[] = [
      listen<{ id: string; data: string; end: number }>('pty-data', (e) => {
        if (e.payload.id !== id) return
        if (replayEnd < 0) pending.push(e.payload)
        else direct(e.payload)
      }),
      listen<string>('pty-exit', (e) => { if (e.payload === id) exited = true }),
    ]
    void Promise.all(un).then(start)

    return () => {
      cancelAnimationFrame(raf)
      ro.disconnect()
      mq.removeEventListener('change', onScheme)
      void Promise.all(un).then((fs) => fs.forEach((f) => f()))
      term.dispose() // la session reste vivante : elle ne se ferme qu'avec le bloc
    }
  })

  function setMode(m: string) {
    if (m === 'wsl') update({ kind: 'wsl', start: undefined, last: undefined, launch: undefined, title: undefined })
    else update({ kind: 'powershell', start: m, last: undefined, launch: undefined, title: undefined, path: p.kind === 'powershell' ? p.path ?? '/home' : '/home' })
  }
</script>

<div class="term">
  <div class="bar">
    <span class="label" title={spec.program + ' ' + spec.args.join(' ')}>{spec.label}</span>
    <span class="grow"></span>
    <Button variant="ghost" onclick={() => (settings = !settings)} aria-expanded={settings}>{t('blocks.terminal.settings')}</Button>
    <Button variant="ghost" onclick={restart}>{t('blocks.terminal.restart')}</Button>
  </div>
  {#if settings}
    <div class="settings">
      <label>{t('blocks.terminal.type')}
        <select value={mode} onchange={(e) => setMode(e.currentTarget.value)}>
          <option value="wsl">{t('blocks.terminal.modeWsl')}</option>
          <option value="distro">{t('blocks.terminal.modeDistro')}</option>
          <option value="profile">{t('blocks.terminal.modeProfile')}</option>
        </select>
      </label>
      {#if mode === 'distro'}
        <label>{t('blocks.terminal.linuxDir')}
          <input value={p.kind === 'powershell' ? p.path ?? '/home' : ''} onchange={(e) => update({ path: e.currentTarget.value, last: undefined })} />
        </label>
      {:else if mode === 'wsl'}
        <label>{t('blocks.terminal.startDir')}
          <input value={p.kind === 'wsl' ? p.cwd ?? '' : ''} placeholder="~" onchange={(e) => update({ cwd: e.currentTarget.value, last: undefined })} />
        </label>
      {/if}
      <p class="hint">{t('blocks.terminal.hint', { last: p.last ?? t('blocks.terminal.none') })}</p>
    </div>
  {/if}
  {#if failure}<div class="fail">{failure}</div>{/if}
  <div class="host" bind:this={host} onpointerdown={() => term.focus()} role="presentation"></div>
  {#if exited}
    <div class="ended">{t('blocks.terminal.ended')} <Button onclick={restart}>{t('blocks.terminal.restart')}</Button></div>
  {/if}
</div>

<style>
  .term { display: flex; flex-direction: column; height: 100%; min-height: 8rem; }
  .bar { display: flex; align-items: center; gap: var(--sp-1); padding: 0 var(--sp-2); border-bottom: 1px solid var(--border); font-size: var(--fs-sm); }
  .label { color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grow { flex: 1; }
  .host { flex: 1; min-height: 0; padding: 4px 0 0 6px; background: var(--surface-1); overflow: hidden; }
  .host :global(.xterm) { height: 100%; }
  .settings { display: grid; gap: var(--sp-2); padding: var(--sp-2) var(--sp-3); border-bottom: 1px solid var(--border); font-size: var(--fs-sm); }
  label { display: grid; gap: 2px; }
  .hint { margin: 0; color: var(--text-dim); }
  .fail { padding: var(--sp-2) var(--sp-3); background: color-mix(in srgb, var(--danger) 14%, transparent); font-size: var(--fs-sm); white-space: pre-wrap; }
  .ended { display: flex; align-items: center; gap: var(--sp-2); padding: var(--sp-2) var(--sp-3); border-top: 1px solid var(--border); font-size: var(--fs-sm); }
</style>
