#!/usr/bin/env node

import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readdirSync } from 'node:fs'

const roles = [
  ['page', '--page'],
  ['surface', '--surface'],
  ['surface_component', '--surface-component'],
  ['button_default_background', '--button-default-background'],
  ['button_default_color', '--button-default-color'],
  ['button_primary_background', '--button-primary-background'],
  ['button_primary_color', '--button-primary-color'],
  ['button_secondary_background', '--button-secondary-background'],
  ['button_secondary_color', '--button-secondary-color'],
  ['button_secondary_border', '--button-secondary-border-color'],
  ['button_danger_background', '--button-danger-background'],
  ['button_danger_color', '--button-danger-color'],
  ['text_primary', '--text-primary'],
  ['text_secondary', '--text-secondary'],
  ['border_default', '--border-default'],
  ['focus_ring', '--focus-ring-color'],
  ['accent_solid', '--accent-solid'],
  ['accent_text', '--accent-text'],
  ['accent_subtle', '--accent-subtle'],
  ['accent_subtle_text', '--accent-subtle-text'],
  ['on_accent', '--on-accent'],
]
const profiles = [
  ['LIGHT_AZURE', 'light/azure'],
  ['DARK_AZURE', 'dark/azure'],
  ['LIGHT_NEUTRAL', 'light/neutral'],
  ['DARK_NEUTRAL', 'dark/neutral'],
]
const profileIds = profiles.map(([, profile]) => profile)
// Tinted neutral ladders swapped in through `[data-neutral]`. The inventory resolves the default
// (achromatic) ladder only, so the tones are resolved here and stored as overrides of it.
const tones = ['slate', 'stone', 'haze']

const args = process.argv.slice(2)
const designIndex = args.indexOf('--design-system')
const designSystem = designIndex >= 0 ? args[designIndex + 1] : null
const mode = args.includes('--write') ? 'write' : args.includes('--check') ? 'check' : null
if (!mode) throw new Error('usage: node tools/sync-yue-theme.mjs [--design-system <path>] --write|--check')
const outputPath = resolve('crates/yue-component/src/theme/generated.rs')
const snapshotPath = resolve('crates/yue-component/tokens/inventory.json')
const hash = value => createHash('sha256').update(value).digest('hex')
let snapshot
if (designSystem) {
  const designRoot = resolve(designSystem)
  const inventoryRun = spawnSync('node', [join(designRoot, 'tools/token-inventory.mjs'), '--json'], { cwd: designRoot, encoding: 'utf8' })
  if (inventoryRun.status !== 0) throw new Error(inventoryRun.stderr || 'token inventory failed')
  const inventory = JSON.parse(inventoryRun.stdout)
  const { createResolver, loadTokenSheet } = await import(pathToFileURL(join(designRoot, 'tools/lib/css-tokens.mjs')).href)
  const resolver = createResolver(loadTokenSheet({ entry: join(designRoot, 'packages/tokens/src/index.css') }))
  const toneValues = {}
  for (const tone of tones) {
    toneValues[tone] = []
    for (const token of inventory.tokens) {
      const values = profileIds.map(id => {
        const [theme, accent] = id.split('/')
        const outcome = resolver.tryValue(token.name, { id, theme, accent, neutral: tone })
        if (!outcome.ok) throw new Error(`${token.name}/${id}/${tone}: ${outcome.error}`)
        return outcome.value
      })
      if (values.some((value, i) => value !== token.resolved[profileIds[i]])) toneValues[tone].push([token.name, values])
    }
  }
  const pkg = JSON.parse(readFileSync(join(designRoot, 'packages/tokens/package.json'), 'utf8'))
  const sourceFiles = []
  function visit(dir) {
    for (const entry of readdirSync(join(designRoot, dir), { withFileTypes: true }).sort((a,b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
      const name = `${dir}/${entry.name}`
      if (entry.isDirectory()) visit(name)
      else sourceFiles.push([name, hash(readFileSync(join(designRoot, name)))])
    }
  }
  visit('packages/tokens')
  for (const name of ['tools/token-inventory.mjs', 'tools/lib/css-tokens.mjs']) sourceFiles.push([name, hash(readFileSync(join(designRoot, name)))])
  const revision = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: designRoot, encoding: 'utf8' })
  const status = spawnSync('git', ['status', '--porcelain', '--', 'packages/tokens', 'tools/token-inventory.mjs', 'tools/lib/css-tokens.mjs'], { cwd: designRoot, encoding: 'utf8' })
  const payload = { inventory, toneValues }
  snapshot = { schemaVersion: 1, source: { package: pkg.name, version: pkg.version, revision: revision.status === 0 ? revision.stdout.trim() : null, dirty: status.status === 0 ? Boolean(status.stdout.trim()) : null, contentSha256: hash(JSON.stringify(sourceFiles)), files: sourceFiles }, payloadSha256: hash(JSON.stringify(payload)), ...payload }
  if (mode === 'write') { mkdirSync(resolve('crates/yue-component/tokens'), { recursive: true }); writeFileSync(snapshotPath, JSON.stringify(snapshot, null, 2) + '\n') }
  else {
    // Identity is the content, not where it came from: a commit or a clean
    // worktree that leaves every hashed file alone is still the same source.
    const stored = existsSync(snapshotPath) ? JSON.parse(readFileSync(snapshotPath, 'utf8')) : null
    if (!stored || stored.payloadSha256 !== snapshot.payloadSha256 || stored.source?.contentSha256 !== snapshot.source.contentSha256) throw new Error('token source differs from checked-in snapshot; refresh with --design-system <path> --write')
    if (stored.source.revision !== snapshot.source.revision || stored.source.dirty !== snapshot.source.dirty) console.warn(`note: snapshot was taken at ${stored.source.revision}${stored.source.dirty ? ' (dirty)' : ''}; source is now ${snapshot.source.revision}${snapshot.source.dirty ? ' (dirty)' : ''} with identical content`)
  }
} else {
  snapshot = JSON.parse(readFileSync(snapshotPath, 'utf8'))
}
if (snapshot.schemaVersion !== 1 || hash(JSON.stringify({ inventory: snapshot.inventory, toneValues: snapshot.toneValues })) !== snapshot.payloadSha256) throw new Error('invalid token snapshot integrity')
const inventory = snapshot.inventory
const identifiers = new Set()
for (const [index, token] of inventory.tokens.entries()) {
  if (!/^--[a-z][a-z0-9-]*$/.test(token.name)) throw new Error(`invalid token name ${token.name}`)
  if (index && inventory.tokens[index - 1].name >= token.name) throw new Error(`token inventory must be sorted and unique: ${token.name}`)
  const identifier = token.name.slice(2).replaceAll('-', '_').toUpperCase()
  if (identifiers.has(identifier)) throw new Error(`duplicate Rust token identifier ${identifier}`)
  identifiers.add(identifier)
  for (const profile of profileIds) {
    if (typeof token.resolved[profile] !== 'string') throw new Error(`missing resolved token ${token.name}/${profile}`)
  }
}
const tokenByName = new Map(inventory.tokens.map(token => [token.name, token]))
for (const tone of tones) {
  const entries = snapshot.toneValues[tone]
  if (!Array.isArray(entries)) throw new Error(`missing neutral overrides ${tone}`)
  for (const [index, [name, values]] of entries.entries()) {
    if (!tokenByName.has(name) || (index && entries[index - 1][0] >= name)) throw new Error(`neutral overrides must be sorted, unique and known: ${tone}/${name}`)
    if (values.length !== profileIds.length || values.some(value => typeof value !== 'string')) throw new Error(`incomplete neutral override ${tone}/${name}`)
  }
}
const toneResolved = new Map(tones.map(tone => [tone, new Map(snapshot.toneValues[tone])]))
const resolvedFor = (tone, profile, tokenName) => toneResolved.get(tone)?.get(tokenName)?.[profileIds.indexOf(profile)] ?? tokenByName.get(tokenName)?.resolved[profile]

function parseHex(profile, tokenName, tone = 'neutral') {
  if (!tokenByName.has(tokenName)) throw new Error(`missing token ${tokenName}`)
  const raw = resolvedFor(tone, profile, tokenName)
  const match = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(raw)
  if (!match) throw new Error(`${tokenName} for ${profile} is not a simple hex color: ${raw}`)
  const hex = match[1].length === 3 ? [...match[1]].map((part) => part + part).join('') : match[1]
  return `0x${hex.toLowerCase()}`
}

// The tokens package this file was generated from. GPUI ships the generated file, not a
// dependency on the web repository, so this line is the only record of which release the
// Rust values match; bump the package before syncing, not after.
const tokensPackage = { name: snapshot.source.package, version: snapshot.source.version }

const lines = [
  '// Generated from the Yue Design System Token inventory tool.',
  `// Source: ${tokensPackage.name}@${tokensPackage.version}`,
  `// Source content SHA-256: ${snapshot.source.contentSha256}` ,
  '// Run `node tools/sync-yue-theme.mjs --write` to regenerate from the checked-in snapshot.',
  'use super::TokenValue;',
  '',
]

const rustString = (value) => JSON.stringify(String(value)).replaceAll('\\/', '/')

// Values which need a font, foreground, or a CSS-only capability stay explicit
// contextual values; they are never silently treated as a different unit.
function colorValue(value) {
  const hex = /^#([\da-f]{3}|[\da-f]{6})$/i.exec(value)
  if (hex) return Number.parseInt(hex[1].length === 3 ? [...hex[1]].map(c => c+c).join('') : hex[1], 16) * 256 + 255
  if (value === 'transparent') return 0
  const mix = /^color-mix\(in srgb, (#[\da-f]+) (?:calc\(([\d.]+)\*100%\)|([\d.]+)%), (#[\da-f]+|transparent)\)$/i.exec(value)
  if (mix) {
    const a = colorValue(mix[1]), b = colorValue(mix[4]), amount = mix[2] === undefined ? Number(mix[3]) / 100 : Number(mix[2])
    // Only 3- and 6-digit hex operands are understood; anything else would
    // otherwise fall through as null and come out black.
    if (a === null || b === null) throw new Error(`unsupported color mix operand ${value}`)
    if (!Number.isFinite(amount) || amount < 0 || amount > 1) throw new Error(`invalid color mix ${value}`)
    if (mix[4] === 'transparent') return a - 255 + Math.round(amount * 255)
    const channels = [24, 16, 8].map(shift => Math.round(((a >>> shift) & 255) * amount + ((b >>> shift) & 255) * (1-amount)))
    return channels[0]*0x1000000 + channels[1]*0x10000 + channels[2]*0x100 + 255
  }
  return null
}
function typed(value) {
  const number = value => { const n = Number(value); if (!Number.isFinite(n)) throw new Error(`non-finite token ${value}`); return `${n}${Number.isInteger(n) ? '.0' : ''}` }
  const color = colorValue(value)
  if (color !== null) return `TokenValue::Color(0x${color.toString(16).padStart(8, '0')})`
  if (/^-?(?:\d*\.)?\d+$/.test(value)) return `TokenValue::Number(${number(value)})`
  let match = /^(-?[\d.]+)(px|ms)$/.exec(value)
  if (match) return `TokenValue::${match[2] === 'px' ? 'Pixels' : 'Milliseconds'}(${number(match[1])})`
  match = /^calc\(\s*([\d.]+)(px|ms)\s*([*/])\s*([\d.]+)\s*\)$/.exec(value)
  if (match) return `TokenValue::${match[2] === 'px' ? 'Pixels' : 'Milliseconds'}(${number(match[3] === '*' ? Number(match[1])*Number(match[4]) : Number(match[1])/Number(match[4]))})`
  // The menu row radius is explicitly constrained to the panel's inner radius.
  // Support this resolved length expression, rather than substituting another
  // component's radius or treating an unsupported expression as zero.
  match = /^max\(\s*(-?[\d.]+)px\s*,\s*calc\(\s*(-?[\d.]+)px\s*([+-])\s*(-?[\d.]+)px\s*\)\s*\)$/.exec(value)
  if (match) return `TokenValue::Pixels(${number(Math.max(Number(match[1]), Number(match[2]) + (match[3] === '+' ? 1 : -1)*Number(match[4])))})`
  match = /^cubic-bezier\(([^)]+)\)$/.exec(value)
  if (match) { const p = match[1].split(','); if (p.length !== 4) throw new Error(value); return `TokenValue::CubicBezier([${p.map(number).join(', ')}])` }
  if (value === 'linear') return 'TokenValue::CubicBezier([0.0, 0.0, 1.0, 1.0])'
  return `TokenValue::Contextual(${rustString(value)})`
}

lines.push('/// Stable names generated from the complete token inventory.', 'pub mod keys {', '    use super::super::TokenKey;')
for (const [index, token] of inventory.tokens.entries()) {
  lines.push(`    pub const ${token.name.slice(2).replaceAll('-', '_').toUpperCase()}: TokenKey = TokenKey(${index});`)
}
lines.push('}', '#[rustfmt::skip]', `pub(super) static VALUES: [[TokenValue; ${profileIds.length}]; ${inventory.tokens.length}] = [`)
for (const token of inventory.tokens) lines.push(`    [${profileIds.map(profile => typed(token.resolved[profile])).join(', ')}],`)
lines.push('];', '')

lines.push(`pub(super) const TOKEN_COUNT: usize = ${inventory.tokens.length};`)
lines.push('#[rustfmt::skip]')
lines.push(`pub(super) static TOKENS: [(&str, &str, &str, [&str; ${profileIds.length}]); ${inventory.tokens.length}] = [`)
for (const token of inventory.tokens) {
  const values = profileIds.map((profile) => rustString(token.resolved[profile]))
  lines.push(`    (${rustString(token.name)}, ${rustString(token.namespace)}, ${rustString(token.layer)}, [${values.join(', ')}]),`)
}
lines.push('];', '')

for (const tone of tones) {
  const overrides = [...toneResolved.get(tone)]
  lines.push(`/// Tokens whose value changes under \`[data-neutral=${tone}]\`, per profile.`)
  lines.push('#[rustfmt::skip]')
  lines.push(`pub(super) static ${tone.toUpperCase()}_TOKENS: [(&str, [&str; ${profileIds.length}]); ${overrides.length}] = [`)
  for (const [name, values] of overrides) {
    lines.push(`    (${rustString(name)}, [${values.map(rustString).join(', ')}]),`)
  }
  lines.push('];', '')
  lines.push('#[rustfmt::skip]', `pub(super) static ${tone.toUpperCase()}_VALUES: [[TokenValue; ${profileIds.length}]; ${overrides.length}] = [`)
  for (const [, values] of overrides) lines.push(`    [${values.map(typed).join(', ')}],`)
  lines.push('];', '')
  for (const [constant, profile] of profiles) {
    const values = roles.map(([, tokenName]) => parseHex(profile, tokenName, tone))
    lines.push(`pub(super) const ${tone.toUpperCase()}_${constant}: [u32; ${roles.length}] = [`)
    lines.push(`    ${values.slice(0, 9).join(', ')},`)
    lines.push(`    ${values.slice(9, 18).join(', ')},`)
    lines.push(`    ${values.slice(18).join(', ')},`)
    lines.push('];', '')
  }
}

for (const [constant, profile] of profiles) {
  const values = roles.map(([, tokenName]) => parseHex(profile, tokenName))
  lines.push(`pub(super) const ${constant}: [u32; ${roles.length}] = [`)
  lines.push(`    ${values.slice(0, 9).join(', ')},`)
  lines.push(`    ${values.slice(9, 18).join(', ')},`)
  lines.push(`    ${values.slice(18).join(', ')},`)
  lines.push('];', '')
  const hover = tokenByName.get('--opacity-hover')?.resolved[profile]
  const pressed = tokenByName.get('--opacity-pressed')?.resolved[profile]
  const hoverValue = Number.parseFloat(hover)
  const pressedValue = Number.parseFloat(pressed)
  if (!Number.isFinite(hoverValue) || !Number.isFinite(pressedValue)) throw new Error(`invalid opacity token values for ${profile}`)
  const f32Literal = value => `${value}${Number.isInteger(value) ? '.0' : ''}`
  lines.push(`pub(super) const ${constant}_HOVER_OPACITY: f32 = ${f32Literal(hoverValue)};`)
  lines.push(`pub(super) const ${constant}_PRESSED_OPACITY: f32 = ${f32Literal(pressedValue)};`, '')
}
const generated = `${lines.join('\n')}`
const current = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : ''
if (mode === 'check') {
  if (current !== generated) {
    console.error(`theme mapping is stale: ${outputPath}`)
    process.exit(1)
  }
  console.log('Yue GPUI theme mapping is current')
} else {
  writeFileSync(outputPath, generated)
  console.log(`wrote ${outputPath}`)
}
