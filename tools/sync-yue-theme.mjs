#!/usr/bin/env node

import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { spawnSync } from 'node:child_process'

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

function usage() {
  console.error('usage: node tools/sync-yue-theme.mjs --design-system <path> [--write|--check]')
  process.exit(2)
}

const args = process.argv.slice(2)
const designIndex = args.indexOf('--design-system')
const designSystem = designIndex >= 0 ? args[designIndex + 1] : null
const mode = args.includes('--write') ? 'write' : args.includes('--check') ? 'check' : null
if (!designSystem || !mode) usage()

const designRoot = resolve(designSystem)
const inventoryCommand = join(designRoot, 'tools', 'token-inventory.mjs')
const outputPath = resolve('crates/yue-component/src/theme/generated.rs')
if (!existsSync(inventoryCommand)) throw new Error(`missing token inventory tool: ${inventoryCommand}`)

const inventoryRun = spawnSync('node', [inventoryCommand, '--json'], {
  cwd: designRoot,
  encoding: 'utf8',
})
if (inventoryRun.status !== 0) throw new Error(inventoryRun.stderr || 'token inventory failed')
const inventory = JSON.parse(inventoryRun.stdout)
const tokenByName = new Map(inventory.tokens.map((token) => [token.name, token]))

function parseHex(profile, tokenName) {
  const token = tokenByName.get(tokenName)
  if (!token) throw new Error(`missing token ${tokenName}`)
  const raw = token.resolved[profile]
  const match = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(raw)
  if (!match) throw new Error(`${tokenName} for ${profile} is not a simple hex color: ${raw}`)
  const hex = match[1].length === 3 ? [...match[1]].map((part) => part + part).join('') : match[1]
  return `0x${hex.toLowerCase()}`
}

const lines = [
  '// Generated from the Yue Design System Token inventory tool.',
  '// Run `node tools/sync-yue-theme.mjs --design-system <path> --write` to regenerate.',
  '',
]

const rustString = (value) => JSON.stringify(String(value)).replaceAll('\\/', '/')

lines.push(`pub(super) const TOKEN_COUNT: usize = ${inventory.tokens.length};`)
lines.push('#[rustfmt::skip]')
lines.push(`pub(super) static TOKENS: [(&str, &str, &str, [&str; ${profileIds.length}]); ${inventory.tokens.length}] = [`)
for (const token of inventory.tokens) {
  const values = profileIds.map((profile) => rustString(token.resolved[profile]))
  lines.push(`    (${rustString(token.name)}, ${rustString(token.namespace)}, ${rustString(token.layer)}, [${values.join(', ')}]),`)
}
lines.push('];', '')

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
  lines.push(`pub(super) const ${constant}_HOVER_OPACITY: f32 = ${hoverValue};`)
  lines.push(`pub(super) const ${constant}_PRESSED_OPACITY: f32 = ${pressedValue};`, '')
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
