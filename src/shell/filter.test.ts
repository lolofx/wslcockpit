import test from 'node:test'
import assert from 'node:assert/strict'
import { filterCommands, type Command } from './filter.ts'

const c = (label: string): Command => ({ id: label, label, run: () => {} })
const list = [c('Démarrer Ubuntu'), c('Arrêter Ubuntu'), c('Démarrer Debian'), c('Tout arrêter')]

test('filtre : mots multiples, accents et casse ignorés', () => {
  assert.deepEqual(filterCommands(list, 'demarrer ubu').map((x) => x.label), ['Démarrer Ubuntu'])
  assert.deepEqual(filterCommands(list, 'ARRETER').map((x) => x.label), ['Arrêter Ubuntu', 'Tout arrêter'])
  assert.equal(filterCommands(list, '').length, 4)
  assert.equal(filterCommands(list, 'zzz').length, 0)
})
