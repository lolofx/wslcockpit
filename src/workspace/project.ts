// Aides pures du bloc Projet (testables avec `node --test`).
export type GitFile = { path: string; x: string; y: string; from: string | null }

export function groupStatus(files: GitFile[]) {
  return {
    staged: files.filter((f) => f.x !== ' ' && f.x !== '?'),
    changed: files.filter((f) => f.y !== ' ' && f.x !== '?'),
    untracked: files.filter((f) => f.x === '?'),
  }
}

const NAMES: Record<string, string> = { M: 'M', A: 'A', D: 'D', R: 'R', C: 'C', U: 'U', T: 'T', '?': '?' }
// Lettre à afficher pour un fichier dans le groupe indexé (colonne X) ou modifié (colonne Y).
export const statusLabel = (f: GitFile, staged: boolean): string =>
  f.x === '?' ? '?' : NAMES[staged ? f.x : f.y] ?? (staged ? f.x : f.y)

export const joinRel = (rel: string, name: string) => (rel ? `${rel}/${name}` : name)

// Fil d'Ariane : la racine, puis chaque dossier avec son chemin relatif.
export function breadcrumb(rel: string): { name: string; rel: string }[] {
  const out = [{ name: '·', rel: '' }]
  let acc = ''
  for (const part of rel.split('/').filter(Boolean)) {
    acc = joinRel(acc, part)
    out.push({ name: part, rel: acc })
  }
  return out
}
