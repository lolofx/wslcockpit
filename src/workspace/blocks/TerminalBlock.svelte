<script lang="ts">
  import type { BlockContext } from '../registry'
  import { terminalSpec, type TerminalProps } from '../terminals'
  let { distro, block, update }: BlockContext = $props()
  const p = $derived(block.props as unknown as TerminalProps)
  const spec = $derived(terminalSpec(distro, p))
  const mode = $derived(p.kind === 'wsl' ? 'wsl' : p.start === 'profile' ? 'profile' : 'distro')
  function setMode(m: string) {
    if (m === 'wsl') update({ kind: 'wsl', start: undefined })
    else update({ kind: 'powershell', start: m, path: p.kind === 'powershell' ? p.path ?? '/home' : '/home' })
  }
</script>

<div class="pad">
  <label>Type
    <select value={mode} onchange={(e) => setMode(e.currentTarget.value)}>
      <option value="wsl">Shell WSL</option>
      <option value="distro">PowerShell, disque de la distro</option>
      <option value="profile">PowerShell, dossier utilisateur</option>
    </select>
  </label>
  {#if mode === 'distro'}
    <label>Dossier Linux
      <input value={p.kind === 'powershell' ? p.path ?? '/home' : ''} onchange={(e) => update({ path: e.currentTarget.value })} />
    </label>
  {:else if mode === 'wsl'}
    <label>Dossier de départ
      <input value={p.kind === 'wsl' ? p.cwd ?? '' : ''} placeholder="~" onchange={(e) => update({ cwd: e.currentTarget.value })} />
    </label>
  {/if}
  <pre>{spec.program} {spec.args.join(' ')}
cwd : {spec.cwd ?? '(dossier utilisateur Windows)'}</pre>
  <p class="hint">Terminal réel (ConPTY + xterm.js) : phase 7.</p>
</div>

<style>
  .pad { padding: 0.75rem; display: grid; gap: 0.5rem; }
  label { display: grid; gap: 0.2rem; font-size: 0.85rem; }
  pre { margin: 0; padding: 0.5rem; background: var(--surface-2); overflow: auto; font-size: 0.8rem; }
  .hint { margin: 0; opacity: 0.6; font-size: 0.8rem; }
</style>
