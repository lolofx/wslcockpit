<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { open as openDialog } from '@tauri-apps/plugin-dialog'
  import type { BlockContext } from '../registry'
  import { parseRoot, type ProjectRoot } from '../model'
  import { isMarkdown } from '../markdown'
  import { groupStatus, statusLabel, breadcrumb, joinRel, type GitFile } from '../project'
  import Button from '../../ui/Button.svelte'
  import { t } from '../../i18n/index.svelte'

  let { block, update, root: viewRoot, open }: BlockContext = $props()
  const own = $derived(parseRoot((block.props as { root?: unknown }).root))
  const root = $derived<ProjectRoot | undefined>(own ?? viewRoot)

  type Status = { branch: string; upstream: string | null; ahead: number; behind: number; files: GitFile[] }
  type Entry = { name: string; dir: boolean }

  let tab = $state<'git' | 'files'>('git')
  let status = $state<Status | null>(null)
  let entries = $state<Entry[]>([])
  let rel = $state('')
  let error = $state('')
  let loading = $state(false)
  let shown = $state<{ title: string; text: string } | null>(null)
  let input = $state('')

  const code = (e: unknown) => String(e)
  function explain(e: unknown): string {
    const s = code(e)
    if (s === 'GIT_NOT_FOUND') return t('blocks.error.gitNotFound')
    if (s === 'NOT_REPO') return t('blocks.error.notRepo')
    if (s === 'WSL_ABSENT') return t('blocks.error.wslAbsent')
    return s
  }

  async function loadStatus() {
    if (!root) return
    try {
      status = await invoke<Status>('git_status', { root })
      error = ''
    } catch (e) {
      status = null
      error = explain(e)
    }
  }
  async function loadDir() {
    if (!root) return
    try {
      entries = await invoke<Entry[]>('fs_list', { root, rel })
      if (tab === 'files') error = ''
    } catch (e) {
      entries = []
      if (tab === 'files') error = explain(e)
    }
  }
  async function refresh() {
    if (!root || loading) return
    loading = true
    await Promise.all([loadStatus(), loadDir()])
    loading = false
  }

  async function showDiff(f: GitFile, staged: boolean) {
    try {
      const text = await invoke<string>('git_diff', { root, path: f.path, staged })
      shown = { title: `${staged ? t('blocks.project.staged') : t('blocks.project.changed')} · ${f.path}`, text: text || t('blocks.project.noDiff') }
    } catch (e) {
      shown = { title: f.path, text: explain(e) }
    }
  }
  async function showFile(path: string) {
    if (isMarkdown(path) && open && root) {
      open('markdown', { root, rel: path })
      return
    }
    try {
      shown = { title: path, text: await invoke<string>('fs_read', { root, rel: path }) }
    } catch (e) {
      const s = code(e)
      shown = { title: path, text: s === 'BINARY' ? t('blocks.error.binary') : s === 'TOO_BIG' ? t('blocks.error.tooBig') : s }
    }
  }
  const openEntry = (e: Entry) => {
    if (e.dir) { rel = joinRel(rel, e.name); shown = null; void loadDir() }
    else void showFile(joinRel(rel, e.name))
  }
  function up(to: string) { rel = to; shown = null; void loadDir() }

  async function saveRoot(text: string) {
    const r = await invoke<ProjectRoot | null>('project_parse', { text })
    if (!r) { error = t('blocks.project.badRoot'); return }
    error = ''
    update({ root: r })
  }
  async function browse() {
    const r = await openDialog({ multiple: false, directory: true })
    if (typeof r === 'string') await saveRoot(r)
  }

  // Rafraîchi à l'ouverture, au retour sur la fenêtre et sur demande (pas de minuterie).
  $effect(() => {
    void root?.distro
    void root?.path
    rel = ''
    shown = null
    void refresh()
  })
  onMount(() => {
    const on = () => document.visibilityState === 'visible' && void refresh()
    window.addEventListener('focus', on)
    return () => window.removeEventListener('focus', on)
  })

  const groupTitle = (g: string) => (g === 'staged' ? t('blocks.project.staged') : g === 'changed' ? t('blocks.project.changed') : t('blocks.project.untracked'))
  const groups = $derived(status ? groupStatus(status.files) : null)
</script>

{#if !root}
  <div class="empty">
    <p>{t('blocks.project.rootIntro')} <code>Debian:/home/moi/projet</code>, {t('blocks.project.rootPath')} <code>\\wsl.localhost\…</code> {t('blocks.project.rootOr')} <code>C:\Dev\projet</code>.</p>
    <div class="row">
      <input bind:value={input} placeholder="Debian:/home/moi/projet" onkeydown={(e) => e.key === 'Enter' && void saveRoot(input)} />
      <Button variant="primary" disabled={!input.trim()} onclick={() => void saveRoot(input)}>{t('blocks.project.use')}</Button>
      <Button onclick={browse}>{t('blocks.project.browse')}</Button>
    </div>
    {#if error}<p class="err">{error}</p>{/if}
  </div>
{:else}
  <div class="proj">
    <div class="bar">
      <button class="tab" class:on={tab === 'git'} onclick={() => (tab = 'git')}>Git{#if status && status.files.length} ({status.files.length}){/if}</button>
      <button class="tab" class:on={tab === 'files'} onclick={() => (tab = 'files')}>{t('blocks.project.files')}</button>
      <span class="grow"></span>
      {#if status}<span class="dim" title={status.upstream ?? ''}>⎇ {status.branch}{#if status.ahead} ↑{status.ahead}{/if}{#if status.behind} ↓{status.behind}{/if}</span>{/if}
      <Button variant="ghost" onclick={() => void refresh()} disabled={loading} title={t('blocks.project.refresh')} aria-label={t('blocks.project.refresh')}>⟳</Button>
      {#if own}<Button variant="ghost" onclick={() => update({ root: undefined })} title={t('blocks.project.change')} aria-label={t('blocks.project.change')}>✎</Button>{/if}
    </div>
    <div class="path dim" title="{root.distro ? root.distro + ':' : ''}{root.path}">{root.distro ? root.distro + ':' : ''}{root.path}</div>
    {#if error && (tab === 'files' || !status)}<p class="err">{error}</p>{/if}
    <div class="split">
      <div class="list">
        {#if tab === 'git' && groups}
          {#each [['staged', groups.staged, true], ['changed', groups.changed, false], ['untracked', groups.untracked, false]] as [group, list, staged] (group)}
            {#if (list as GitFile[]).length}
              <h4>{groupTitle(group as string)} ({(list as GitFile[]).length})</h4>
              {#each list as GitFile[] as f (f.path + group)}
                <button class="row-f" onclick={() => (group === 'untracked' ? void showFile(f.path) : void showDiff(f, staged as boolean))}>
                  <b class="st">{statusLabel(f, staged as boolean)}</b><span class="nm">{f.path}</span>
                </button>
              {/each}
            {/if}
          {/each}
          {#if status && status.files.length === 0}<p class="dim pad">{t('blocks.project.clean')}</p>{/if}
        {:else if tab === 'files'}
          <div class="crumbs">
            {#each breadcrumb(rel) as c, i}
              {#if i > 0}<span class="dim">/</span>{/if}
              <button class="lnk" onclick={() => up(c.rel)}>{c.name}</button>
            {/each}
          </div>
          {#each entries as e (e.name)}
            <button class="row-f" onclick={() => openEntry(e)}><b class="st">{e.dir ? '▸' : ''}</b><span class="nm">{e.name}</span></button>
          {/each}
        {/if}
      </div>
      {#if shown}
        <div class="view">
          <div class="vh"><span>{shown.title}</span><Button variant="ghost" onclick={() => (shown = null)} aria-label={t('blocks.project.close')}>✕</Button></div>
          <pre>{shown.text}</pre>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .empty { padding: var(--sp-3); display: grid; gap: var(--sp-2); font-size: var(--fs-sm); }
  .row { display: flex; gap: var(--sp-1); }
  .row input { flex: 1; min-width: 0; font: inherit; padding: var(--sp-2); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); }
  .proj { display: flex; flex-direction: column; height: 100%; min-height: 8rem; font-size: var(--fs-sm); }
  .bar { display: flex; align-items: center; gap: var(--sp-1); padding: 0 var(--sp-2); border-bottom: 1px solid var(--border); }
  .tab { font: inherit; padding: var(--sp-1) var(--sp-2); border: 0; border-bottom: 2px solid transparent; background: none; color: var(--text); cursor: pointer; }
  .tab.on { border-bottom-color: var(--accent); font-weight: 600; }
  .grow { flex: 1; }
  .dim { color: var(--text-dim); }
  .path { padding: 0 var(--sp-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.75rem; }
  .split { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .list { overflow: auto; flex: 1; min-height: 4rem; }
  h4 { margin: var(--sp-2) var(--sp-3) var(--sp-1); font-size: 0.75rem; color: var(--text-dim); text-transform: uppercase; }
  .row-f { display: flex; gap: var(--sp-2); width: 100%; text-align: left; font: inherit; padding: 2px var(--sp-3); border: 0; background: none; color: var(--text); cursor: pointer; }
  .row-f:hover, .row-f:focus-visible { background: var(--surface-2); outline: none; }
  .st { width: 1.4rem; flex: none; font-family: monospace; color: var(--accent); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .crumbs { display: flex; gap: var(--sp-1); padding: var(--sp-1) var(--sp-3); flex-wrap: wrap; }
  .lnk { border: 0; background: none; color: var(--accent); cursor: pointer; font: inherit; padding: 0; }
  .view { flex: 1; min-height: 6rem; display: flex; flex-direction: column; border-top: 1px solid var(--border); }
  .vh { display: flex; justify-content: space-between; align-items: center; padding: 0 var(--sp-3); background: var(--surface-2); }
  pre { margin: 0; padding: var(--sp-2) var(--sp-3); overflow: auto; flex: 1; font-family: 'Cascadia Mono', Consolas, monospace; font-size: 0.75rem; white-space: pre; }
  .err { margin: var(--sp-2) var(--sp-3); padding: var(--sp-2); background: color-mix(in srgb, var(--danger) 14%, transparent); border-radius: var(--radius); white-space: pre-wrap; }
  .pad { padding: var(--sp-2) var(--sp-3); }
</style>
