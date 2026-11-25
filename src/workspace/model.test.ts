import test from 'node:test'
import assert from 'node:assert/strict'
import { parseDoc, savePreset, applyPreset, deletePreset, emptyDoc, defaultLayout, moveBlock, resizeAt, removeBlock, addBlock, makeBlock, parseLayout, findBlock, MAX_STACKS } from './model.ts'
import { terminalSpec, wslUncPath, psQuote, psEncode, parseOsc7, parseOsc9, OSC7_PROMPT_COMMAND, PS_TRACK } from './terminals.ts'
const decodePs = (b64: string) => Buffer.from(b64, 'base64').toString('utf16le')

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
  assert.deepEqual(p.args.slice(0, 3), ['-NoLogo', '-NoExit', '-EncodedCommand'])
  assert.ok(decodePs(p.args[3]).startsWith("Set-Location -LiteralPath '\\\\wsl.localhost\\Ubuntu\\home\\lolofx'; "))
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

import { addView, updateView, removeView, moveView, duplicateView, viewKey, blockDistro, emptyDoc as _e, parseDoc as _p, makeBlock as _mb } from './model.ts'

test('affichages : ajout, renommage, ordre, copie, suppression', () => {
  let { doc, id } = addView(_e(), '  Projet  ')
  assert.equal(doc.views[0].name, 'Projet')
  assert.ok(doc.distros[viewKey(id)])
  doc = updateView(doc, id, { name: '   ', icon: { kind: 'emoji', value: '🚀' } })
  assert.equal(doc.views[0].name, 'Projet')
  assert.equal(doc.views[0].icon?.value, '🚀')
  const b = addView(doc, 'B')
  doc = moveView(b.doc, b.id, -1)
  assert.deepEqual(doc.views.map((v) => v.name), ['B', 'Projet'])
  assert.equal(moveView(doc, b.id, -1), doc)
  const c = duplicateView(doc, id)!
  assert.equal(c.doc.views.length, 3)
  assert.notEqual(c.id, id)
  doc = removeView(c.doc, id)
  assert.equal(doc.views.length, 2)
  assert.equal(doc.distros[viewKey(id)], undefined)
})

test('affichages : lecture tolérante', () => {
  const d = _p({ version: 1, distros: {}, presets: {}, views: [{ id: 'a', name: 'X', icon: { kind: 'emoji', value: '🐧' }, root: { distro: 'Debian', path: '/srv' } }, { id: 'a', name: 'dup' }, { name: 'sans id' }, 5] })
  assert.equal(d.views.length, 1)
  assert.equal(d.views[0].icon?.value, '🐧')
  assert.deepEqual(d.views[0].root, { distro: 'Debian', path: '/srv' })
  assert.deepEqual(_p({ version: 1, distros: {}, presets: {} }).views, [])
})

test('distro d un bloc', () => {
  assert.equal(blockDistro(_mb('terminal', { distro: 'Debian' }), 'X'), 'Debian')
  assert.equal(blockDistro(_mb('terminal', {}), 'X'), 'X')
})

test('suivi du dossier : OSC 7 et OSC 9;9', () => {
  assert.equal(parseOsc7('file://Debian/home/lolofx/mon%20projet'), '/home/lolofx/mon projet')
  assert.equal(parseOsc7('file://Debian/home/a b%zz'), '/home/a b%zz')
  assert.equal(parseOsc7('http://x/y'), null)
  assert.equal(parseOsc9('9;C:\\Users\\a'), 'C:\\Users\\a')
  assert.equal(parseOsc9('4;1;50'), null)
  assert.equal(parseOsc9('9;'), null)
})

test('reprise au dossier courant mémorisé', () => {
  const w = terminalSpec('Ubuntu', { kind: 'wsl', cwd: '~', last: '/srv/app' })
  assert.deepEqual(w.args, ['-d', 'Ubuntu', '--cd', '/srv/app'])
  assert.deepEqual(w.env, [['PROMPT_COMMAND', OSC7_PROMPT_COMMAND]])
  const pr = terminalSpec('Ubuntu', { kind: 'powershell', start: 'profile', last: "C:\\Users\\o'neil" })
  assert.ok(decodePs(pr.args[3]).startsWith("Set-Location -LiteralPath 'C:\\Users\\o''neil'; "))
  assert.ok(decodePs(pr.args[3]).endsWith(PS_TRACK))
  const pd = terminalSpec('Ubuntu', { kind: 'powershell', start: 'distro', path: '/home', last: 'C:\\x' })
  assert.ok(decodePs(pd.args[3]).startsWith("Set-Location -LiteralPath 'C:\\x'; "))
  assert.equal(decodePs(psEncode('é😀a')), 'é😀a')
})

import { groupStatus, statusLabel, joinRel, breadcrumb } from './project.ts'
test('projet : groupes git, libellés, fil d’Ariane', () => {
  const f = (path: string, x: string, y: string) => ({ path, x, y, from: null })
  const g = groupStatus([f('a', ' ', 'M'), f('b', 'A', ' '), f('c', 'M', 'M'), f('d', '?', '?')])
  assert.deepEqual(g.staged.map((x) => x.path), ['b', 'c'])
  assert.deepEqual(g.changed.map((x) => x.path), ['a', 'c'])
  assert.deepEqual(g.untracked.map((x) => x.path), ['d'])
  assert.equal(statusLabel(f('c', 'A', 'M'), true), 'A')
  assert.equal(statusLabel(f('c', 'A', 'M'), false), 'M')
  assert.equal(statusLabel(f('d', '?', '?'), false), '?')
  assert.equal(joinRel('', 'x'), 'x')
  assert.equal(joinRel('a/b', 'x'), 'a/b/x')
  assert.deepEqual(breadcrumb('a/b'), [{ name: '·', rel: '' }, { name: 'a', rel: 'a' }, { name: 'b', rel: 'a/b' }])
})

import { resolveLink, isMarkdown, dirOf } from './markdown.ts'
test('markdown : liens', () => {
  assert.deepEqual(resolveLink('a/b.md', 'https://x.io/p'), { kind: 'external', url: 'https://x.io/p' })
  assert.deepEqual(resolveLink('a/b.md', '#titre%20un'), { kind: 'anchor', id: 'titre un' })
  assert.deepEqual(resolveLink('a/b.md', 'c.md'), { kind: 'file', rel: 'a/c.md' })
  assert.deepEqual(resolveLink('a/b.md', '../d/e.md#x'), { kind: 'file', rel: 'd/e.md' })
  assert.deepEqual(resolveLink('b.md', 'docs/mon%20fichier.md'), { kind: 'file', rel: 'docs/mon fichier.md' })
  assert.deepEqual(resolveLink('a/b.md', '/README.md'), { kind: 'file', rel: 'README.md' })
  assert.equal(resolveLink('b.md', '../secret').kind, 'refused')
  assert.equal(resolveLink('b.md', 'javascript:alert(1)').kind, 'refused')
  assert.equal(resolveLink('b.md', 'file:///etc/passwd').kind, 'refused')
  assert.equal(resolveLink('b.md', 'C:\\x').kind, 'refused')
  assert.equal(resolveLink('b.md', '//evil.io/x').kind, 'refused')
  assert.ok(isMarkdown('README.MD') && isMarkdown('a.markdown') && !isMarkdown('a.txt'))
  assert.equal(dirOf('a/b/c.md'), 'a/b')
  assert.equal(dirOf('c.md'), '')
})

import { dockBlock, undockBlock, parseViewLayout } from './model.ts'
test('dock : mettre de côté, remettre, conserver à travers les opérations', () => {
  let l = defaultLayout()
  const all = l.stacks.flatMap((s) => s.blocks)
  const id = all[0].id
  l = dockBlock(l, id)
  assert.equal(l.dock?.length, 1)
  assert.equal(l.stacks.flatMap((s) => s.blocks).length, all.length - 1)
  l = addBlock(l, makeBlock('log'))
  assert.equal(l.dock?.length, 1, 'ajouter un bloc garde le dock')
  l = moveBlock(l, l.stacks[0].blocks[0].id, { kind: 'new-stack', at: 0 })
  assert.equal(l.dock?.length, 1, 'déplacer garde le dock')
  assert.equal(parseLayout(JSON.parse(JSON.stringify(l))).dock?.length, 1)
  const back = undockBlock(l, id)
  assert.equal(back.dock, undefined)
  assert.ok(back.stacks.flatMap((s) => s.blocks).some((b) => b.id === id))
  assert.equal(removeBlock(dockBlock(back, id), id).dock, undefined)
})

test('dock : tout mis de côté reste lisible, un affichage invalide devient vide', () => {
  let l = defaultLayout()
  for (const b of l.stacks.flatMap((s) => s.blocks)) l = dockBlock(l, b.id)
  assert.equal(l.stacks.length, 0)
  assert.equal(l.dock?.length, 4)
  const r = parseLayout(JSON.parse(JSON.stringify(l)))
  assert.equal(r.stacks.length, 0)
  assert.equal(r.dock?.length, 4)
  assert.equal(parseViewLayout({ version: 2 }).stacks.length, 0)
  assert.equal(parseViewLayout(null).dock, undefined)
  assert.equal(parseViewLayout({ version: 1, stacks: [{ id: 1, blocks: [] }] }).stacks.length, 0)
})

import { parseLaunchers, upsertLauncher, removeLauncher, availableIn, launchProps, suggestions, emptyLaunchers } from '../launchers/model.ts'
test('lanceurs : lecture, ajout, disponibilité, props de bloc, suggestions', () => {
  const d = parseLaunchers({ launchers: [
    { id: 'a', name: ' Claude ', target: 'wsl', command: ' claude ', distro: 'Debian', cwd: ' ' },
    { id: 'a', name: 'doublon', target: 'wsl', command: 'x' },
    { id: 'b', name: 'PS', target: 'windows', command: 'aspire run', distro: 'ignorée' },
    { id: 'c', name: '', target: 'wsl', command: 'x' },
    { id: 'd', name: 'x', target: 'autre', command: 'x' },
  ] })
  assert.deepEqual(d.launchers.map((l) => l.id), ['a', 'b'])
  assert.deepEqual(d.launchers[0], { id: 'a', name: 'Claude', target: 'wsl', command: 'claude', distro: 'Debian' })
  assert.equal(d.launchers[1].distro, undefined)
  const [cl, ps] = d.launchers
  assert.ok(availableIn(cl, 'Debian') && !availableIn(cl, 'Ubuntu') && availableIn(cl, null) && availableIn(ps, 'Ubuntu'))
  assert.deepEqual(launchProps(cl, 'X', { distro: 'Debian', path: '/srv/p' }), { kind: 'wsl', distro: 'Debian', launch: 'claude', title: 'Claude', cwd: '/srv/p' })
  assert.equal(launchProps(cl, 'X', { distro: 'Autre', path: '/srv/p' }).cwd, undefined)
  assert.deepEqual(launchProps(ps, 'X', { distro: '', path: 'C:\\Dev' }), { kind: 'powershell', start: 'profile', launch: 'aspire run', title: 'PS', dir: 'C:\\Dev' })
  assert.deepEqual(launchProps({ ...cl, distro: undefined, cwd: '/tmp' }, 'Ubuntu'), { kind: 'wsl', distro: 'Ubuntu', launch: 'claude', title: 'Claude', cwd: '/tmp' })
  let doc = upsertLauncher(emptyLaunchers(), { id: 'z', name: 'Z', target: 'wsl', command: 'codex' })
  doc = upsertLauncher(doc, { id: 'z', name: 'Z2', target: 'wsl', command: 'codex' })
  assert.equal(doc.launchers.length, 1)
  assert.equal(doc.launchers[0].name, 'Z2')
  assert.equal(upsertLauncher(doc, { id: 'q', name: '', target: 'wsl', command: '' }), doc)
  assert.equal(removeLauncher(doc, 'z').launchers.length, 0)
  assert.deepEqual(suggestions(doc, 'Debian', ['claude', 'codex', 'vim']), [{ tool: 'claude', name: 'Claude Code' }])
})

import { LAUNCH_SCRIPT } from './terminals.ts'
test('lanceur dans un terminal : script fixe, commande par l\'environnement', () => {
  const w = terminalSpec('Debian', { kind: 'wsl', launch: 'claude --resume', cwd: '/srv/p' })
  assert.deepEqual(w.args, ['-d', 'Debian', '--cd', '/srv/p', '-e', 'sh', '-c', LAUNCH_SCRIPT])
  assert.ok(!LAUNCH_SCRIPT.includes('"') && !LAUNCH_SCRIPT.includes('claude'))
  assert.deepEqual(w.env.map((e) => e[0]), ['PROMPT_COMMAND', 'WSLC_CMD'])
  assert.equal(w.env[1][1], 'claude --resume')
  const p = terminalSpec('Debian', { kind: 'powershell', start: 'profile', launch: 'aspire run', dir: 'C:\\Dev' })
  const s = decodePs(p.args[3])
  assert.ok(s.startsWith("Set-Location -LiteralPath 'C:\\Dev'; ") && s.endsWith('; aspire run'))
  assert.ok(terminalSpec('D', { kind: 'wsl' }).args.indexOf('-e') < 0)
})
