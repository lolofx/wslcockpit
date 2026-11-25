<script lang="ts">
  import type { BlockContext } from '../registry'
  import { app, find, setPolicy, size, stateLabel } from '../../state/app.svelte'
  import Badge from '../../ui/Badge.svelte'
  import { t } from '../../i18n/index.svelte'
  let { distro }: BlockContext = $props()
  const d = $derived(find(distro))
</script>

<div class="pad">
  {#if d}
    <div class="row">
      <Badge tone={d.state === 'running' ? 'success' : 'neutral'}>{stateLabel(d.state)}</Badge>
      <Badge>WSL{d.version}</Badge>
      {#if d.is_default}<Badge tone="accent">{t('blocks.state.default')}</Badge>{/if}
    </div>
    <dl>
      {#if d.vhdx_bytes !== null}<dt>VHDX</dt><dd>{size(d.vhdx_bytes)}</dd>{/if}
      {#if app.mon?.disks[d.name]}<dt>{t('blocks.state.disk')}</dt><dd>{size(app.mon.disks[d.name][0])} / {size(app.mon.disks[d.name][1])}</dd>{/if}
    </dl>
    <label>{t('blocks.state.policy')}
      <select value={app.policies[d.name] ?? 'manual'} onchange={(e) => setPolicy(d.name, e.currentTarget.value)}>
        <option value="manual">{t('blocks.state.manual')}</option>
        <option value="start">{t('blocks.state.start')}</option>
        <option value="keepalive">{t('blocks.state.keepalive')}</option>
      </select>
    </label>
  {:else}
    <p>{t('blocks.state.notFound')}</p>
  {/if}
</div>

<style>
  .pad { padding: var(--sp-3); display: grid; gap: var(--sp-3); }
  .row { display: flex; gap: var(--sp-1); flex-wrap: wrap; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: var(--sp-1) var(--sp-3); margin: 0; font-size: var(--fs-sm); }
  dt { color: var(--text-dim); }
  dd { margin: 0; }
  label { display: grid; gap: var(--sp-1); font-size: var(--fs-sm); color: var(--text-dim); }
  select { font: inherit; padding: var(--sp-2); border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface-1); color: var(--text); }
</style>
