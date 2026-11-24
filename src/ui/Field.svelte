<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements'
  type Props = HTMLInputAttributes & { label: string; hint?: string; value?: string }
  let { label, hint, value = $bindable(''), ...rest }: Props = $props()
</script>

<label class="field">
  <span class="l">{label}</span>
  <input bind:value {...rest} />
  {#if hint}<span class="hint">{hint}</span>{/if}
</label>

<style>
  .field { display: grid; gap: var(--sp-1); font-size: var(--fs-sm); }
  .l { color: var(--text-dim); }
  input { font: inherit; font-size: var(--fs); padding: var(--sp-2) var(--sp-3); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); }
  input:focus-visible { outline: none; border-color: var(--accent); box-shadow: var(--focus); }
  .hint { color: var(--text-dim); }
</style>
