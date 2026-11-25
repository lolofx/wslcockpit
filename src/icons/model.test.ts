import test from 'node:test'
import assert from 'node:assert/strict'
import { initialsOf, cleanInitials, parseSpec, parseIcons, setDistroIcon, emptyIcons, resolveIcon, guessLogo } from './model.ts'

test('initiales', () => {
  assert.equal(initialsOf('Ubuntu-24.04'), 'U2')
  assert.equal(initialsOf('kali linux'), 'KL')
  assert.equal(initialsOf('debian'), 'DE')
  assert.equal(initialsOf('é'), 'É')
  assert.equal(initialsOf(''), '')
  assert.equal(cleanInitials('  abc '), 'AB')
})

test('parseSpec rejette le invalide, garde la couleur valide', () => {
  assert.equal(parseSpec(null), null)
  assert.equal(parseSpec({ kind: 'x', value: 'a' }), null)
  assert.equal(parseSpec({ kind: 'emoji', value: '' }), null)
  assert.deepEqual(parseSpec({ kind: 'logo', value: 'debian', color: '#aabbcc' }), { kind: 'logo', value: 'debian', color: '#aabbcc' })
  assert.deepEqual(parseSpec({ kind: 'logo', value: 'debian', color: 'red' }), { kind: 'logo', value: 'debian' })
  assert.deepEqual(parseSpec({ kind: 'initials', value: 'abc' }), { kind: 'initials', value: 'AB' })
})

test('document tolérant, ajout et retrait', () => {
  assert.deepEqual(parseIcons('oups'), emptyIcons())
  const d = parseIcons({ distros: { a: { kind: 'emoji', value: '🐧' }, b: 3 } })
  assert.deepEqual(Object.keys(d.distros), ['a'])
  const d2 = setDistroIcon(d, 'c', { kind: 'symbol', value: 'box' })
  assert.equal(d2.distros.c.value, 'box')
  assert.equal(setDistroIcon(d2, 'c', null).distros.c, undefined)
})

test('résolution et suggestion', () => {
  assert.deepEqual(resolveIcon(null, 'Debian'), { kind: 'initials', value: 'DE' })
  assert.equal(guessLogo('Ubuntu-24.04'), 'ubuntu')
  assert.equal(guessLogo('machine'), null)
})
