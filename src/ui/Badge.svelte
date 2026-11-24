<script lang="ts">
  import type { Snippet } from 'svelte'
  let { tone = 'neutral', children }: { tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent'; children: Snippet } = $props()
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
  .badge { display: inline-block; font-size: var(--fs-sm); padding: 1px var(--sp-2); border-radius: 999px; background: var(--surface-3); color: var(--text-dim); white-space: nowrap; }
  .success { background: color-mix(in srgb, var(--success) 18%, transparent); color: var(--success); }
  .warning { background: color-mix(in srgb, var(--warning) 18%, transparent); color: var(--warning); }
  .danger { background: color-mix(in srgb, var(--danger) 18%, transparent); color: var(--danger); }
  .accent { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--accent); }
</style>
