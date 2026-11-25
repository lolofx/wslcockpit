// Spécification d'un terminal : pure, sans exécution ; passée au pseudo-terminal (ConPTY) par pty_open.
// `wsl`        : shell dans la distro (wsl.exe -d <distro> --cd <dossier>).
// `powershell` : PowerShell Windows, hors WSL, dont le dossier de départ est soit le disque de la distro
//                vu depuis Windows (\\wsl.localhost\<distro>\...), soit le profil utilisateur.
// `last` : dernier dossier courant connu du shell (suivi par OSC 7 / OSC 9;9), repris au relancement de l'appli.
// Forme native : chemin Linux pour `wsl`, chemin Windows ou UNC pour `powershell`.
export type TerminalProps =
  | { kind: 'wsl'; cwd?: string; last?: string }
  | { kind: 'powershell'; start: 'distro' | 'profile'; path?: string; last?: string }

export type TerminalSpec = { program: string; args: string[]; cwd: string | null; label: string; env: [string, string][] }

// Suivi du dossier courant : le shell écrit son dossier à chaque invite, sans toucher aux fichiers de l'utilisateur.
// bash : PROMPT_COMMAND passé par l'environnement (WSLENV, géré côté Rust). zsh et fish : pas de suivi.
export const OSC7_PROMPT_COMMAND = String.raw`printf '\033]7;file://%s%s\033\\' "$HOSTNAME" "$PWD"`
// PowerShell : fonction prompt enveloppée, OSC 9;9 (convention Windows Terminal) avec le chemin brut.
export const PS_TRACK = [
  '$global:__wc_prompt = $function:prompt',
  "function global:prompt { [Console]::Write([char]27 + ']9;9;' + (Get-Location).ProviderPath + [char]27 + '\\'); & $global:__wc_prompt }",
].join('; ')

// -EncodedCommand : UTF-16LE en base64, évite tout problème de guillemets sur la ligne de commande.
export function psEncode(script: string): string {
  let bin = ''
  for (const ch of script) {
    const c = ch.codePointAt(0)!
    const units = c > 0xffff ? [0xd800 + ((c - 0x10000) >> 10), 0xdc00 + ((c - 0x10000) & 0x3ff)] : [c]
    for (const u of units) bin += String.fromCharCode(u & 0xff, u >> 8)
  }
  return btoa(bin)
}

// `data` d'un OSC 7 : file://hôte/chemin (le chemin peut être encodé en pourcentage ou non).
export function parseOsc7(data: string): string | null {
  const m = /^file:\/\/[^/]*(\/.*)$/.exec(data)
  if (!m) return null
  try { return decodeURIComponent(m[1]) } catch { return m[1] }
}

// `data` d'un OSC 9 : seul `9;<chemin>` (suivi de dossier PowerShell) nous intéresse.
export function parseOsc9(data: string): string | null {
  return data.startsWith('9;') && data.length > 2 ? data.slice(2) : null
}

export function wslUncPath(distro: string, linuxPath: string): string {
  const p = linuxPath.startsWith('/') ? linuxPath : `/${linuxPath}`
  return `\\\\wsl.localhost\\${distro}${p.replace(/\//g, '\\')}`.replace(/\\+$/, '') || `\\\\wsl.localhost\\${distro}`
}

export function terminalSpec(distro: string, props: TerminalProps): TerminalSpec {
  if (props.kind === 'wsl') {
    const args = ['-d', distro]
    const dir = props.last || props.cwd
    if (dir) args.push('--cd', dir)
    return { program: 'wsl.exe', args, cwd: null, label: `${distro}${dir ? ` ${dir}` : ''}`, env: [['PROMPT_COMMAND', OSC7_PROMPT_COMMAND]] }
  }
  if (props.start === 'profile') {
    const go = props.last ? `Set-Location -LiteralPath ${psQuote(props.last)}; ` : ''
    return {
      program: 'powershell.exe',
      args: ['-NoLogo', '-NoExit', '-EncodedCommand', psEncode(go + PS_TRACK)],
      cwd: null,
      label: `PowerShell ${props.last ?? '(profil)'}`,
      env: [],
    }
  }
  // Le dossier de départ UNC (\\wsl.localhost\...) n'est pas fiable comme dossier de processus :
  // PowerShell s'y place lui-même au démarrage.
  const unc = props.last || wslUncPath(distro, props.path || '/home')
  return {
    program: 'powershell.exe',
    args: ['-NoLogo', '-NoExit', '-EncodedCommand', psEncode(`Set-Location -LiteralPath ${psQuote(unc)}; ${PS_TRACK}`)],
    cwd: null,
    label: `PowerShell → ${unc}`,
    env: [],
  }
}

// Chaîne PowerShell entre apostrophes : seule l'apostrophe est spéciale (doublée).
export function psQuote(s: string): string {
  return `'${s.replace(/'/g, "''")}'`
}
