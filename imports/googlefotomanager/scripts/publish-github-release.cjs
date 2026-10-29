#!/usr/bin/env node
const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { publishReleaseFiles, readVersion } = require('./rename-artifacts.cjs')

const version = process.argv[2] || readVersion()
const notesFile = process.argv[3]

if (!notesFile) {
  console.error('Usage: node scripts/publish-github-release.cjs <version> <notes-file>')
  process.exit(1)
}

const files = publishReleaseFiles(version)
for (const filePath of files) {
  if (!fs.existsSync(filePath)) {
    console.error(`Missing release file: ${filePath}`)
    process.exit(1)
  }
}

const notes = fs.readFileSync(notesFile, 'utf8')
const args = [
  'release',
  'create',
  `v${version}`,
  ...files,
  '--repo',
  'sebastianoboem/GoogleFotoManager',
  '--title',
  `v${version}`,
  '--notes',
  notes
]

console.log(
  'Uploading:',
  files.map((filePath) => path.basename(filePath)).join(', ')
)

const result = spawnSync('gh', args, { stdio: 'inherit' })
process.exit(result.status ?? 1)
