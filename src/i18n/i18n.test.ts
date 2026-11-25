import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { detectLang, flatten, interpolate, missingKeys, translate, varsOf, type Dict, type Lang, type Tree } from './core.ts'

const dicts: Record<Lang, Dict> = {
  fr: flatten({ a: { b: 'Bonjour {{name}}', n: { one: '{{count}} fichier', other: '{{count}} fichiers' } }, only: 'fr' }),
  en: flatten({ a: { b: 'Hello {{ name }}', n: { one: '{{count}} file', other: '{{count}} files' } } }),
}

test('flatten et interpolation', () => {
  assert.deepEqual(Object.keys(dicts.fr).sort(), ['a.b', 'a.n', 'only'])
  assert.equal(interpolate('x {{a}} {{b}}', { a: 1 }), 'x 1 {{b}}')
  assert.equal(translate(dicts, 'en', 'a.b', { name: 'Loïc' }), 'Hello Loïc')
})

test('pluriel', () => {
  assert.equal(translate(dicts, 'en', 'a.n', { count: 1 }), '1 file')
  assert.equal(translate(dicts, 'en', 'a.n', { count: 3 }), '3 files')
  assert.equal(translate(dicts, 'fr', 'a.n', { count: 0 }), '0 fichier')
})

test('repli sur le français puis sur la clé', () => {
  assert.equal(translate(dicts, 'en', 'only'), 'fr')
  assert.equal(translate(dicts, 'en', 'nope.key'), 'nope.key')
})

test('langue système', () => {
  assert.equal(detectLang('en-US'), 'en')
  assert.equal(detectLang('fr-FR'), 'fr')
  assert.equal(detectLang('de-DE'), 'fr')
  assert.equal(detectLang(undefined), 'fr')
})

// Les fichiers réels : mêmes clés, mêmes variables dans les deux langues.
function load(lang: Lang): Dict {
  const dir = new URL(`./locales/${lang}/`, import.meta.url)
  const out: Dict = {}
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.json'))) {
    flatten(JSON.parse(readFileSync(new URL(f, dir), 'utf8')) as Tree, f.replace(/\.json$/, ''), out)
  }
  return out
}

test('fr.json et en.json ont les mêmes clés et variables', () => {
  const fr = load('fr')
  const en = load('en')
  assert.deepEqual(missingKeys(fr, en), [], 'clés absentes de en')
  assert.deepEqual(missingKeys(en, fr), [], 'clés absentes de fr')
  for (const k of Object.keys(fr)) assert.deepEqual(varsOf(en[k]), varsOf(fr[k]), `variables différentes : ${k}`)
})

// Toute clé littérale t('ns.cle') utilisée dans src/ existe dans les fichiers.
function walk(dir: URL, out: string[] = []): string[] {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const u = new URL(e.name + (e.isDirectory() ? '/' : ''), dir)
    if (e.isDirectory()) walk(u, out)
    else if (/\.(svelte|ts)$/.test(e.name) && !e.name.endsWith('.test.ts')) out.push(u.pathname)
  }
  return out
}

test('les clés littérales utilisées existent', () => {
  const fr = load('fr')
  const bad: string[] = []
  for (const f of walk(new URL('../', import.meta.url))) {
    const src = readFileSync(f, 'utf8')
    for (const m of src.matchAll(/\bt\(\s*'([a-z0-9_]+(?:\.[A-Za-z0-9_]+)+)'/g)) {
      if (!(m[1] in fr)) bad.push(`${f.split('/src/')[1]}: ${m[1]}`)
    }
  }
  assert.deepEqual(bad, [])
})
