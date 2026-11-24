import type { Component } from 'svelte'
import type { Block } from './model'
import StateBlock from './blocks/StateBlock.svelte'
import LogBlock from './blocks/LogBlock.svelte'
import ConfigBlock from './blocks/ConfigBlock.svelte'
import TerminalBlock from './blocks/TerminalBlock.svelte'

export type BlockContext = { distro: string; block: Block; update: (props: Record<string, unknown>) => void }

// Ajouter un type de bloc = une entrée ici + un composant. Rien d'autre à toucher (layout, glisser, persistance).
export type BlockDef = {
  type: string
  label: string
  component: Component<BlockContext>
  presets: { label: string; props: Record<string, unknown> }[]
}

export const registry: BlockDef[] = [
  { type: 'state', label: 'État', component: StateBlock, presets: [{ label: 'État', props: {} }] },
  { type: 'config', label: 'Configuration', component: ConfigBlock, presets: [{ label: 'Configuration', props: {} }] },
  { type: 'log', label: 'Journal', component: LogBlock, presets: [{ label: 'Journal', props: {} }] },
  {
    type: 'terminal',
    label: 'Terminal',
    component: TerminalBlock,
    presets: [
      { label: 'Terminal WSL', props: { kind: 'wsl' } },
      { label: 'PowerShell sur le disque de la distro', props: { kind: 'powershell', start: 'distro', path: '/home' } },
      { label: 'PowerShell dossier utilisateur Windows', props: { kind: 'powershell', start: 'profile' } },
    ],
  },
]

export const def = (type: string) => registry.find((d) => d.type === type)
