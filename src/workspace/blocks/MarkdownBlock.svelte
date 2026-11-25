<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import type { BlockContext } from '../registry'
  import { parseRoot } from '../model'
  import { resolveLink } from '../markdown'
  import Button from '../../ui/Button.svelte'
  import { t } from '../../i18n/index.svelte'

  let { block, update, root: viewRoot }: BlockContext = $props()
  const bp = $derived(block.props as { root?: unknown; rel?: string })
  const root = $derived(parseRoot(bp.root) ?? viewRoot)
  const rel = $derived(bp.rel ?? '')

  let html = $state('')
  let error = $state('')
  let box: HTMLDivElement

  async function load() {
    if (!root || !rel) { html = ''; return }
    try {
      const text = await invoke<string>('fs_read', { root, rel })
      // Le HTML du Markdown est nettoyé : pas de script, pas de gestionnaire d'événement.
      html = DOMPurify.sanitize(await marked.parse(text, { gfm: true, async: true }), { USE_PROFILES: { html: true } })
      error = ''
    } catch (e) {
      const s = String(e)
      html = ''
      error = s === 'BINARY' ? t('blocks.error.binary') : s === 'TOO_BIG' ? t('blocks.error.tooBig') : s
    }
  }

  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a')
    if (!a) return
    e.preventDefault()
    const l = resolveLink(rel, a.getAttribute('href') ?? '')
    if (l.kind === 'external') void invoke('open_external', { url: l.url }).catch((x) => (error = String(x)))
    else if (l.kind === 'file') update({ rel: l.rel })
    else if (l.kind === 'anchor') box.querySelector(`[id="${CSS.escape(l.id)}"]`)?.scrollIntoView()
  }

  $effect(() => {
    void rel
    void root?.path
    void load()
  })
  onMount(() => {
    const on = () => document.visibilityState === 'visible' && void load()
    window.addEventListener('focus', on)
    return () => window.removeEventListener('focus', on)
  })
</script>

<div class="md">
  <div class="bar">
    <span class="t" title={rel}>{rel || t('workspace.type.markdown')}</span>
    <Button variant="ghost" onclick={() => void load()} title={t('blocks.markdown.reload')} aria-label={t('blocks.markdown.reload')}>⟳</Button>
  </div>
  {#if error}<p class="err">{error}</p>{/if}
  {#if !rel}
    <p class="dim">{t('blocks.markdown.hint')}</p>
  {/if}
  <div class="doc" bind:this={box} onclick={click} onkeydown={() => {}} role="presentation">{@html html}</div>
</div>

<style>
  .md { display: flex; flex-direction: column; height: 100%; font-size: var(--fs-sm); }
  .bar { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--sp-2) 0 var(--sp-3); border-bottom: 1px solid var(--border); }
  .t { color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dim { color: var(--text-dim); padding: var(--sp-3); }
  .err { margin: var(--sp-2) var(--sp-3); padding: var(--sp-2); background: color-mix(in srgb, var(--danger) 14%, transparent); border-radius: var(--radius); }
  .doc { flex: 1; overflow: auto; padding: var(--sp-2) var(--sp-4); line-height: 1.55; font-size: var(--fs); }
  .doc :global(h1), .doc :global(h2) { border-bottom: 1px solid var(--border); padding-bottom: 0.2em; }
  .doc :global(code) { background: var(--surface-2); padding: 0.1em 0.35em; border-radius: var(--radius-sm); font-size: 0.9em; }
  .doc :global(pre) { background: var(--surface-2); padding: var(--sp-3); border-radius: var(--radius); overflow: auto; }
  .doc :global(pre code) { background: none; padding: 0; }
  .doc :global(a) { color: var(--accent); }
  .doc :global(blockquote) { margin-left: 0; padding-left: var(--sp-3); border-left: 3px solid var(--border); color: var(--text-dim); }
  .doc :global(table) { border-collapse: collapse; }
  .doc :global(th), .doc :global(td) { border: 1px solid var(--border); padding: 0.25em 0.6em; }
  .doc :global(img) { max-width: 100%; }
</style>
