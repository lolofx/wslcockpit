import test from 'node:test'
import assert from 'node:assert/strict'
import { defaultLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, parseLayout, findBlock, MAX_STACKS } from './model.ts'
import { terminalSpec, wslUncPath } from './terminals.ts'

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
  assert.equal(p.cwd, '\\\\wsl.localhost\\Ubuntu\\home\\lolofx')
  assert.equal(terminalSpec('Ubuntu', { kind: 'powershell', start: 'profile' }).cwd, null)
})
