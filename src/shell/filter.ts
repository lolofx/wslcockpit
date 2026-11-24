export type Command = { id: string; label: string; run: () => void }

// Recherche simple : tous les mots saisis doivent apparaître (insensible à la casse et aux accents).
const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()
export function filterCommands(cmds: Command[], q: string): Command[] {
  const words = fold(q).split(/\s+/).filter(Boolean)
  return cmds.filter((c) => words.every((w) => fold(c.label).includes(w)))
}
