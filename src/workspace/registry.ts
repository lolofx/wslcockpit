import type { Component } from 'svelte'
import type { Block, ProjectRoot } from './model'
import { t } from '../i18n/index.svelte'
import StateBlock from './blocks/StateBlock.svelte'
import LogBlock from './blocks/LogBlock.svelte'
import ConfigBlock from './blocks/ConfigBlock.svelte'
import TerminalBlock from './blocks/TerminalBlock.svelte'
import ProjectBlock from './blocks/ProjectBlock.svelte'
import MarkdownBlock from './blocks/MarkdownBlock.svelte'

// `root` : dossier du projet de l'affichage qui contient le bloc (s'il en a un).
export type BlockContext = { distro: string; block: Block; update: (props: Record<string, unknown>) => void; root?: ProjectRoot; open?: (type: string, props: Record<string, unknown>) => void }

// Ajouter un type de bloc = une entrée ici + un composant. Rien d'autre à toucher (layout, glisser, persistance).
export type BlockDef = {
  type: string
  readonly label: string // résolu à la lecture (getter) : suit la langue
  icon: string // symbole Lucide (src/icons/data.ts) pour la barre des blocs en attente
  component: Component<BlockContext>
  presets: { readonly label: string; props: Record<string, unknown> }[]
}

export const registry: BlockDef[] = [
  { type: 'state', icon: 'layout-dashboard', get label() { return t('workspace.type.state') }, component: StateBlock, presets: [{ get label() { return t('workspace.type.state') }, props: {} }] },
  { type: 'config', icon: 'settings', get label() { return t('workspace.type.config') }, component: ConfigBlock, presets: [{ get label() { return t('workspace.type.config') }, props: {} }] },
  { type: 'log', icon: 'layers', get label() { return t('workspace.type.log') }, component: LogBlock, presets: [{ get label() { return t('workspace.type.log') }, props: {} }] },
  { type: 'project', icon: 'folder-git-2', get label() { return t('workspace.type.project') }, component: ProjectBlock, presets: [{ get label() { return t('workspace.preset.project') }, props: {} }] },
  { type: 'markdown', icon: 'book-open', get label() { return t('workspace.type.markdown') }, component: MarkdownBlock, presets: [{ get label() { return t('workspace.type.markdown') }, props: {} }] },
  {
    type: 'terminal',
    icon: 'terminal',
    get label() { return t('workspace.type.terminal') },
    component: TerminalBlock,
    presets: [
      { get label() { return t('workspace.title.wsl') }, props: { kind: 'wsl' } },
      { get label() { return t('workspace.preset.psDistro') }, props: { kind: 'powershell', start: 'distro', path: '/home' } },
      { get label() { return t('workspace.preset.psProfile') }, props: { kind: 'powershell', start: 'profile' } },
    ],
  },
]

export const def = (type: string) => registry.find((d) => d.type === type)
