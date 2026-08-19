#!/usr/bin/env node
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { readVersion } = require('./rename-artifacts.cjs')

const version = process.argv[2] || readVersion()
const notesFile = process.argv[3]

if (!notesFile) {
  console.error('Usage: node scripts/publish-release.cjs <version> <notes-file>')
  process.exit(1)
}

console.log(`\n→ publish-github-release.cjs ${version}`)
const result = spawnSync(
  process.execPath,
  [path.join(__dirname, 'publish-github-release.cjs'), version, notesFile],
  {
    stdio: 'inherit',
    env: process.env
  }
)
if ((result.status ?? 1) !== 0) {
  process.exit(result.status ?? 1)
}

console.log('\nRelease published on GitHub.')
