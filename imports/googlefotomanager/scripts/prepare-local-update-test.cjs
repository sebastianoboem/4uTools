#!/usr/bin/env node
/**
 * Stage release artifacts for a local updater test and serve them.
 *
 * Usage:
 *   node scripts/prepare-local-update-test.cjs [port]
 *
 * Expects finalize-release output in release/ for the current package.json version.
 */
const fs = require('node:fs')
const path = require('node:path')
const { spawn } = require('node:child_process')
const {
  readVersion,
  releaseDir,
  userReleaseFiles
} = require('./rename-artifacts.cjs')

const port = Number(process.argv[2] || 8765)
const version = readVersion()
const stagingDir = path.join(releaseDir, 'staging-local')

function sha512Base64(filePath) {
  return require('node:crypto')
    .createHash('sha512')
    .update(fs.readFileSync(filePath))
    .digest('base64')
}

fs.mkdirSync(stagingDir, { recursive: true })
for (const name of fs.readdirSync(stagingDir)) {
  if (name.endsWith('.app')) continue
  fs.rmSync(path.join(stagingDir, name), { recursive: true, force: true })
}

const files = userReleaseFiles(version)
const exe = files.find((f) => f.endsWith('.exe'))
const zips = files.filter((f) => f.endsWith('.zip'))
if (!exe || zips.length === 0) {
  console.error('Missing exe/zip in release/ — run npm run build:release first')
  process.exit(1)
}

const { githubAssetName } = require('./rename-artifacts.cjs')

for (const filePath of files.filter((f) => f.endsWith('.zip') || f.endsWith('.exe'))) {
  // Serve GitHub-style names (no spaces) so electron-updater URLs stay valid.
  const destName = githubAssetName(path.basename(filePath))
  fs.copyFileSync(filePath, path.join(stagingDir, destName))
}

const exeName = githubAssetName(path.basename(exe))
const exeSha = sha512Base64(exe)
fs.writeFileSync(
  path.join(stagingDir, 'latest.yml'),
  [
    `version: ${version}`,
    'files:',
    `  - url: ${exeName}`,
    `    sha512: ${exeSha}`,
    `path: ${exeName}`,
    `sha512: ${exeSha}`,
    `releaseDate: '${new Date().toISOString()}'`,
    ''
  ].join('\n')
)

const zipEntries = zips.map((zipPath) => {
  const name = githubAssetName(path.basename(zipPath))
  return { name, sha: sha512Base64(zipPath), zipPath }
})
const primaryZip = zipEntries[0]
const macLines = [`version: ${version}`, 'files:']
for (const entry of zipEntries) {
  macLines.push(`  - url: ${entry.name}`)
  macLines.push(`    sha512: ${entry.sha}`)
}
macLines.push(`path: ${primaryZip.name}`)
macLines.push(`sha512: ${primaryZip.sha}`)
macLines.push(`releaseDate: '${new Date().toISOString()}'`)
macLines.push('')
fs.writeFileSync(path.join(stagingDir, 'latest-mac.yml'), macLines.join('\n'))

console.log(`Staged ${version} → ${stagingDir}`)
console.log(`Feed URL: http://127.0.0.1:${port}/`)
console.log('Serving… (Ctrl+C to stop)')

const child = spawn(
  process.execPath,
  [
    '-e',
    `
const http = require('http');
const fs = require('fs');
const path = require('path');
const root = ${JSON.stringify(stagingDir)};
http.createServer((req, res) => {
  const rel = decodeURIComponent((req.url || '/').split('?')[0].replace(/^\\//, ''));
  const file = path.join(root, rel);
  if (!file.startsWith(root) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    res.writeHead(404); res.end('not found'); return;
  }
  res.writeHead(200);
  fs.createReadStream(file).pipe(res);
}).listen(${port}, '127.0.0.1', () => console.log('listening on http://127.0.0.1:${port}/'));
`
  ],
  { stdio: 'inherit' }
)

child.on('exit', (code) => process.exit(code ?? 0))
process.on('SIGINT', () => child.kill('SIGINT'))
process.on('SIGTERM', () => child.kill('SIGTERM'))
