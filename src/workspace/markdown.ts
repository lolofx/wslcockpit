// Aperçu Markdown : liens et chemins purs (testables) ; le rendu HTML est dans MarkdownBlock.svelte.
export type Link =
  | { kind: 'external'; url: string }
  | { kind: 'anchor'; id: string }
  | { kind: 'file'; rel: string }
  | { kind: 'refused' }

export const isMarkdown = (name: string) => /\.(md|markdown|mdx)$/i.test(name)

// Dossier d'un chemin relatif ('a/b/c.md' -> 'a/b').
export const dirOf = (rel: string) => (rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : '')

// Résout `href` relativement au fichier `from` ; refuse tout ce qui sortirait du projet.
export function resolveLink(from: string, href: string): Link {
  const h = href.trim()
  if (/^https?:\/\//i.test(h)) return { kind: 'external', url: h }
  if (h.startsWith('#')) return { kind: 'anchor', id: decodeURIComponent(h.slice(1)) }
  if (/^[a-z][a-z0-9+.-]*:/i.test(h) || h.startsWith('//')) return { kind: 'refused' } // mailto:, file:, javascript:, C:\...
  const path = h.split(/[?#]/)[0]
  if (!path) return { kind: 'refused' }
  const parts = path.startsWith('/') ? [] : dirOf(from).split('/').filter(Boolean)
  for (const seg of decodeURI(path).split('/')) {
    if (seg === '' || seg === '.') continue
    if (seg === '..') {
      if (!parts.length) return { kind: 'refused' }
      parts.pop()
    } else parts.push(seg)
  }
  return parts.length ? { kind: 'file', rel: parts.join('/') } : { kind: 'refused' }
}
