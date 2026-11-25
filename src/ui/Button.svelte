<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { HTMLButtonAttributes } from 'svelte/elements'

  type Props = HTMLButtonAttributes & {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    icon?: boolean
    children: Snippet
  }
  let { variant = 'secondary', icon = false, children, ...rest }: Props = $props()
</script>

<button class="btn {variant}" class:icon {...rest}>{@render children()}</button>

<style>
  .btn {
    font: inherit; font-size: var(--fs-sm); line-height: 1.2;
    padding: var(--sp-2) var(--sp-3); border-radius: var(--radius);
    border: 1px solid var(--border); background: var(--surface-1); color: var(--text);
    cursor: pointer; transition: background var(--dur), border-color var(--dur), transform var(--dur) var(--ease);
  }
  .btn:active:not(:disabled) { transform: scale(0.97); }
  .btn:hover:not(:disabled) { background: var(--surface-2); }
  .btn:focus-visible { outline: none; box-shadow: var(--focus); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
  .primary:hover:not(:disabled) { background: color-mix(in srgb, var(--accent) 88%, var(--text)); }
  .ghost { border-color: transparent; background: transparent; }
  .danger { background: var(--danger); border-color: var(--danger); color: var(--on-danger); }
  .danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 88%, var(--text)); }
  .icon { padding: var(--sp-2); min-width: 2rem; }
</style>
