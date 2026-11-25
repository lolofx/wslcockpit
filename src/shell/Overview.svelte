<script lang="ts">
  import { app, display, act, size, stateLabel } from '../state/app.svelte'
  import { distroMenu } from './menus'
  import Button from '../ui/Button.svelte'
  import Badge from '../ui/Badge.svelte'
  import Menu from '../ui/Menu.svelte'
  import Icon from '../icons/Icon.svelte'
  import { distroIcon } from '../state/icons.svelte'

  let { open }: { open: (name: string) => void } = $props()
</script>

{#if !app.loaded}
  <p class="dim">Chargement…</p>
{:else if app.distros.length === 0}
  <p class="dim">Aucune distribution installée.</p>
  <Button variant="primary" onclick={() => (app.ops = { mode: 'create', source: '' })}>Nouvelle distribution</Button>
{:else}
  <div class="grid">
    {#each app.distros as d (d.name)}
      {@const running = d.state === 'running'}
      <article class="card">
        <button class="name" onclick={() => open(d.name)} title="Ouvrir l’espace de travail">
          <Icon spec={distroIcon(d.name)} label={display(d.name)} size={26} />
          <span class="dot" class:on={running}></span>
          <strong>{display(d.name)}</strong>
        </button>
        <div class="badges">
          <Badge tone={running ? 'success' : 'neutral'}>{stateLabel(d.state)}</Badge>
          <Badge>WSL{d.version}</Badge>
          {#if d.is_default}<Badge tone="accent">défaut</Badge>{/if}
        </div>
        <div class="meta dim">
          {#if d.vhdx_bytes !== null}VHDX {size(d.vhdx_bytes)}{/if}
          {#if app.mon?.disks[d.name]} · Disque {size(app.mon.disks[d.name][0])} / {size(app.mon.disks[d.name][1])}{/if}
        </div>
        <div class="foot">
          {#if running}
            <Button onclick={() => act('stop', d.name)} disabled={app.busy !== null}>Arrêter</Button>
          {:else}
            <Button variant="primary" onclick={() => act('start', d.name)} disabled={app.busy !== null || d.state === 'installing'}>Démarrer</Button>
          {/if}
          <Menu items={distroMenu(d)} label="Actions de {display(d.name)}" />
        </div>
      </article>
    {/each}
  </div>
{/if}

<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: var(--sp-3); }
  .card { display: grid; gap: var(--sp-2); padding: var(--sp-4); background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-lg); }
  .name { display: flex; align-items: center; gap: var(--sp-2); border: 0; background: none; padding: 0; color: var(--text); font: inherit; cursor: pointer; text-align: left; min-width: 0; }
  .name:hover strong { text-decoration: underline; }
  strong { overflow-wrap: anywhere; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--text-dim); flex: none; }
  .dot.on { background: var(--success); }
  .badges { display: flex; flex-wrap: wrap; gap: var(--sp-1); }
  .meta { font-size: var(--fs-sm); min-height: 1.2em; }
  .dim { color: var(--text-dim); }
  .foot { display: flex; justify-content: space-between; align-items: center; margin-top: var(--sp-1); }
</style>
