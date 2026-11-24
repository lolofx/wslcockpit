// Spécification d'un terminal : pure, sans exécution ; passée au pseudo-terminal (ConPTY) par pty_open.
// `wsl`        : shell dans la distro (wsl.exe -d <distro> --cd <dossier>).
// `powershell` : PowerShell Windows, hors WSL, dont le dossier de départ est soit le disque de la distro
//                vu depuis Windows (\\wsl.localhost\<distro>\...), soit le profil utilisateur.
export type TerminalProps =
  | { kind: 'wsl'; cwd?: string }
  | { kind: 'powershell'; start: 'distro' | 'profile'; path?: string }

export type TerminalSpec = { program: string; args: string[]; cwd: string | null; label: string }

export function wslUncPath(distro: string, linuxPath: string): string {
  const p = linuxPath.startsWith('/') ? linuxPath : `/${linuxPath}`
  return `\\\\wsl.localhost\\${distro}${p.replace(/\//g, '\\')}`.replace(/\\+$/, '') || `\\\\wsl.localhost\\${distro}`
}

export function terminalSpec(distro: string, props: TerminalProps): TerminalSpec {
  if (props.kind === 'wsl') {
    const args = ['-d', distro]
    if (props.cwd) args.push('--cd', props.cwd)
    return { program: 'wsl.exe', args, cwd: null, label: `${distro}${props.cwd ? ` ${props.cwd}` : ''}` }
  }
  if (props.start === 'profile') {
    return { program: 'powershell.exe', args: ['-NoLogo'], cwd: null, label: 'PowerShell (profil)' }
  }
  // Le dossier de départ UNC (\\wsl.localhost\...) n'est pas fiable comme dossier de processus :
  // PowerShell s'y place lui-même au démarrage.
  const unc = wslUncPath(distro, props.path || '/home')
  return {
    program: 'powershell.exe',
    args: ['-NoLogo', '-NoExit', '-Command', `Set-Location -LiteralPath ${psQuote(unc)}`],
    cwd: null,
    label: `PowerShell → ${unc}`,
  }
}

// Chaîne PowerShell entre apostrophes : seule l'apostrophe est spéciale (doublée).
export function psQuote(s: string): string {
  return `'${s.replace(/'/g, "''")}'`
}
