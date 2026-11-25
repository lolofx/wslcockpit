<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { app, display } from '../state/app.svelte'
  import { launchers, saveLauncher, deleteLauncher } from '../state/launchers.svelte'
  import { launcherId, suggestions, KNOWN_TOOLS, type Launcher } from '../launchers/model'
  import { pickIcon } from '../icons/picker.svelte'
  import { confirmAction } from '../ui/feedback.svelte'
  import Icon from '../icons/Icon.svelte'
  import Dialog from '../ui/Dialog.svelte'
  import Button from '../ui/Button.svelte'

  let edit = $state<Launcher | null>(null)
  let detectIn = $state('')
  let found = $state<{ tool: string; name: string }[] | null>(null)
  let busy = $state(false)

  const blank = (): Launcher => ({ id: launcherId(), name: '', target: 'wsl', command: '' })

  async function detect() {
    if (!detectIn) return
    busy = true
    found = null
    try {
      const tools = await invoke<string[]>('detect_tools', { distro: detectIn, names: Object.keys(KNOWN_TOOLS) })
      found = suggestions(launchers.doc, detectIn, tools)
      if (tools.length === 0) found = []
    } finally {
      busy = false
    }
  }
  const addSuggested = (s: { tool: string; name: string }) =>
    saveLauncher({ id: launcherId(), name: s.name, target: 'wsl', distro: detectIn, command: s.tool }).then(() => detect())

  async function chooseIcon() {
    if (!edit) return
    const r = await pickIcon({ title: t('launchers.icon.title'), label: edit.name || '?', current: edit.icon ?? null })
    if (r !== undefined) edit = { ...edit, icon: r ?? undefined }
  }
  async function remove(l: Launcher) {
    if (await confirmAction({ title: t('launchers.removeTitle', { name: l.name }), confirmLabel: t('launchers.delete'), danger: true })) await deleteLauncher(l.id)
  }
  async function submit() {
    if (!edit || !edit.name.trim() || !edit.command.trim()) return
    await saveLauncher(edit)
    edit = null
  }
</script>

<section class="panel">
  <div class="top">
    <h2>{t('launchers.title')}</h2>
    <Button onclick={() => (edit = blank())}>{t('launchers.new')}</Button>
  </div>
  <p class="muted">{t('launchers.intro')}</p>

  {#each launchers.doc.launchers as l (l.id)}
    <div class="item">
      <Icon spec={l.icon} label={l.name} size={24} />
      <div class="txt">
        <strong>{l.name}</strong>
        <span class="muted">{l.target === 'windows' ? t('launchers.targetWindows') : l.distro ? display(l.distro) : t('launchers.anyDistro')} · <code>{l.command}</code>{l.cwd ? ` · ${l.cwd}` : ''}</span>
      </div>
      <Button variant="ghost" onclick={() => (edit = { ...l })}>{t('launchers.edit')}</Button>
      <Button variant="ghost" onclick={() => void remove(l)}>{t('launchers.delete')}</Button>
    </div>
  {:else}
    <p class="muted">{t('launchers.none')}</p>
  {/each}

  <h3>{t('launchers.detect.title')}</h3>
  <div class="row">
    <select bind:value={detectIn} aria-label={t('launchers.detect.distro')}>
      <option value="">{t('launchers.detect.choose')}</option>
      {#each app.distros as d (d.name)}<option value={d.name}>{display(d.name)}</option>{/each}
    </select>
    <Button disabled={!detectIn || busy} onclick={() => void detect()}>{busy ? t('launchers.detect.searching') : t('launchers.detect.search')}</Button>
  </div>
  <p class="muted">{t('launchers.detect.hint')}</p>
  {#if found}
    {#each found as s (s.tool)}
      <div class="item">
        <div class="txt"><strong>{s.name}</strong><span class="muted">{t('launchers.detect.foundIn', { distro: display(detectIn) })}</span></div>
        <Button variant="primary" onclick={() => void addSuggested(s)}>{t('launchers.detect.add')}</Button>
      </div>
    {:else}
      <p class="muted">{t('launchers.detect.nothing')}</p>
    {/each}
  {/if}
</section>

{#if edit}
  <Dialog title={launchers.doc.launchers.some((x) => x.id === edit!.id) ? t('launchers.form.editTitle') : t('launchers.form.newTitle')} width={520} onclose={() => (edit = null)}>
    <div class="form">
      <label>{t('launchers.form.name')} <input bind:value={edit.name} placeholder="Claude Code" /></label>
      <label>{t('launchers.form.runIn')}
        <select bind:value={edit.target}>
          <option value="wsl">{t('launchers.form.targetWsl')}</option>
          <option value="windows">{t('launchers.targetWindows')}</option>
        </select>
      </label>
      {#if edit.target === 'wsl'}
        <label>{t('launchers.form.distro')}
          <select value={edit.distro ?? ''} onchange={(e) => (edit = { ...edit!, distro: e.currentTarget.value || undefined })}>
            <option value="">{t('launchers.form.distroAny')}</option>
            {#each app.distros as d (d.name)}<option value={d.name}>{display(d.name)}</option>{/each}
          </select>
        </label>
      {/if}
      <label>{t('launchers.form.command')} <input bind:value={edit.command} placeholder="claude" /></label>
      <label>{t('launchers.form.cwd')} <input value={edit.cwd ?? ''} onchange={(e) => (edit = { ...edit!, cwd: e.currentTarget.value || undefined })} placeholder={t('launchers.form.cwdPlaceholder')} /></label>
      <div class="ic"><Icon spec={edit.icon} label={edit.name || '?'} size={28} /><Button onclick={() => void chooseIcon()}>{t('launchers.icon.button')}</Button></div>
    </div>
    {#snippet footer()}
      <Button variant="primary" disabled={!edit?.name.trim() || !edit?.command.trim()} onclick={() => void submit()}>{t('launchers.form.save')}</Button>
      <Button onclick={() => (edit = null)}>{t('launchers.form.cancel')}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .panel { display: grid; gap: var(--sp-2); margin-top: var(--sp-5); }
  .top { display: flex; justify-content: space-between; align-items: center; }
  h2, h3 { margin: 0; }
  h3 { margin-top: var(--sp-3); font-size: var(--fs); }
  .muted { color: var(--text-dim); font-size: var(--fs-sm); }
  .item { display: flex; align-items: center; gap: var(--sp-3); padding: var(--sp-2) var(--sp-3); border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface-1); }
  .txt { flex: 1; display: grid; min-width: 0; }
  .row { display: flex; gap: var(--sp-2); }
  .form { display: grid; gap: var(--sp-3); }
  .form label { display: grid; gap: var(--sp-1); font-size: var(--fs-sm); color: var(--text-dim); }
  .form input, .form select, .row select { font: inherit; padding: var(--sp-2) var(--sp-3); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); }
  .ic { display: flex; align-items: center; gap: var(--sp-3); }
</style>
