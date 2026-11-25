<script lang="ts">
  import { LOGOS, SYMBOLS } from './data'
  import { resolveIcon, type IconSpec } from './model'

  let { spec, label = '', size = 22 }: { spec?: IconSpec | null; label?: string; size?: number } = $props()
  const r = $derived(resolveIcon(spec, label))
  const logo = $derived(r.kind === 'logo' ? LOGOS[r.value] : undefined)
  const symbol = $derived(r.kind === 'symbol' ? SYMBOLS[r.value] : undefined)
  // Un logo ou symbole inconnu (fichier édité à la main) retombe sur les initiales.
  const fallback = $derived(r.kind === 'initials' || (r.kind === 'logo' && !logo) || (r.kind === 'symbol' && !symbol))
  const text = $derived(fallback ? (r.kind === 'initials' ? r.value : resolveIcon(null, label).value) : r.value)
</script>

<span class="icon" style:width="{size}px" style:height="{size}px" style:color={r.color} style:font-size="{size * 0.5}px" aria-hidden="true">
  {#if logo}
    <svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor"><path d={logo.d} /></svg>
  {:else if symbol}
    <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html symbol}</svg>
  {:else if r.kind === 'emoji'}
    <span class="emoji" style:font-size="{size * 0.8}px">{r.value}</span>
  {:else}
    <b class="ini" class:colored={!!r.color}>{text}</b>
  {/if}
</span>

<style>
  .icon { display: inline-grid; place-items: center; flex: none; line-height: 1; }
  .emoji { font-family: 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif; }
  .ini { font-weight: 700; letter-spacing: -0.02em; }
</style>
