import test from 'node:test'
import assert from 'node:assert/strict'
import { parseDoc, savePreset, applyPreset, deletePreset, emptyDoc, defaultLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, parseLayout, findBlock, MAX_STACKS } from './model.ts'
import { terminalSpec, wslUncPath, psQuote } from './terminals.ts'

const sum = (xs: { size: number }[]) => xs.reduce((a, x) => a + x.size, 0)

test('défaut : 2 piles, tailles normalisées', () => {
  const l = defaultLayout()
  assert.equal(l.stacks.length, 2)
  assert.ok(Math.abs(sum(l.stacks) - 1) < 1e-9)
  l.stacks.forEach((s) => assert.ok(Math.abs(sum(s.blocks) - 1) < 1e-9))
})

test('resizeAt respecte le minimum et conserve le total', () => {
  const items = [{ size: 0.5 }, { size: 0.5 }]
  const r = resizeAt(items, 0, 10)
  assert.ok(Math.abs(sum(r) - 1) < 1e-9)
  assert.ok(r[1].size >= 0.1 - 1e-9)
  assert.equal(resizeAt(items, 5, 0.1), items)
})

test('déplacer un bloc vers une autre pile', () => {
  const l = defaultLayout()
  const id = l.stacks[0].blocks[0].id
  const m = moveBlock(l, id, { kind: 'stack', stackId: l.stacks[1].id, index: 0 })
  assert.equal(m.stacks[1].blocks[0].id, id)
  assert.equal(m.stacks[0].blocks.length, 1)
})

test('réordonner dans la même pile', () => {
  const l = defaultLayout()
  const [a, b] = l.stacks[0].blocks
  const m = moveBlock(l, a.id, { kind: 'stack', stackId: l.stacks[0].id, index: 2 })
  assert.deepEqual(m.stacks[0].blocks.map((x) => x.id), [b.id, a.id])
})

test('vider une pile la supprime ; nouvelle pile plafonnée', () => {
  let l = defaultLayout()
  for (const b of [...l.stacks[0].blocks]) l = removeBlock(l, b.id)
  assert.equal(l.stacks.length, 1)
  l = addBlock(l, makeBlock('log'))
  while (l.stacks.length < MAX_STACKS) {
    const id = l.stacks[0].blocks[0].id
    l = addBlock(l, makeBlock('log'), 0)
    l = moveBlock(l, id, { kind: 'new-stack', at: l.stacks.length })
  }
  const before = l
  const id = l.stacks[0].blocks[0].id
  assert.equal(moveBlock(l, id, { kind: 'new-stack', at: 0 }), before)
  assert.ok(findBlock(l, id))
})

test('disposition corrompue = défaut', () => {
  assert.equal(parseLayout(null).stacks.length, 2)
  assert.equal(parseLayout({ version: 2 }).stacks.length, 2)
  assert.equal(parseLayout({ version: 1, stacks: [{ id: 1 }] }).stacks.length, 2)
  const ok = parseLayout(JSON.parse(JSON.stringify(defaultLayout())))
  assert.equal(ok.stacks.length, 2)
})

test('terminaux : chemin UNC et specs', () => {
  assert.equal(wslUncPath('Ubuntu', '/home/lolofx/proj'), '\\\\wsl.localhost\\Ubuntu\\home\\lolofx\\proj')
  assert.equal(wslUncPath('Ubuntu', '/'), '\\\\wsl.localhost\\Ubuntu')
  const w = terminalSpec('Ubuntu', { kind: 'wsl', cwd: '~' })
  assert.deepEqual(w.args, ['-d', 'Ubuntu', '--cd', '~'])
  const p = terminalSpec('Ubuntu', { kind: 'powershell', start: 'distro', path: '/home/lolofx' })
  assert.equal(p.program, 'powershell.exe')
  assert.equal(p.cwd, null)
  assert.deepEqual(p.args.slice(0, 3), ['-NoLogo', '-NoExit', '-Command'])
  assert.equal(p.args[3], "Set-Location -LiteralPath '\\\\wsl.localhost\\Ubuntu\\home\\lolofx'")
  assert.equal(psQuote("o'neil"), "'o''neil'")
  assert.equal(terminalSpec('Ubuntu', { kind: 'powershell', start: 'profile' }).cwd, null)
})

test('document : valeurs invalides retombent sur un document vide', () => {
  assert.deepEqual(parseDoc(null), emptyDoc())
  assert.deepEqual(parseDoc({ version: 2 }), emptyDoc())
  assert.deepEqual(parseDoc('x'), emptyDoc())
})

test('preset : sauvegarde, application avec nouveaux ids, suppression', () => {
  const l = defaultLayout()
  const doc = savePreset(emptyDoc(), '  Dev  ', l)
  assert.deepEqual(Object.keys(doc.presets), ['Dev'])
  const a = applyPreset(doc, 'Dev')!
  const ids = (x: typeof l) => x.stacks.flatMap((s) => [s.id, ...s.blocks.map((b) => b.id)])
  assert.equal(ids(a).length, ids(l).length)
  assert.ok(ids(a).every((i) => !ids(l).includes(i)))
  assert.equal(applyPreset(doc, 'absent'), null)
  assert.equal(savePreset(doc, '   ', l), doc)
  assert.deepEqual(deletePreset(doc, 'Dev').presets, {})
})

test('document : une disposition corrompue est réparée', () => {
  const d = parseDoc({ version: 1, distros: { U: { version: 9 } }, presets: {} })
  assert.equal(d.distros.U.version, 1)
  assert.ok(d.distros.U.stacks.length > 0)
})
