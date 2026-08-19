#!/usr/bin/env node
/**
 * Build an older-version packaged app for local updater tests.
 * Bakes GFM_UPDATE_FEED_URL into the main bundle (no post-sign plist hacks).
 *
 * Usage:
 *   node scripts/build-local-update-client.cjs <old-version>
 */
const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')

const oldVersion = process.argv[2]
if (!oldVersion) {
  console.error('Usage: node scripts/build-local-update-client.cjs <old-version>')
  process.exit(1)
}

const root = path.join(__dirname, '..')
const pkgPath = path.join(root, 'package.json')
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
const currentVersion = pkg.version
// Keep the test .app outside the feed directory — Squirrel replaces the running
// bundle in place after a successful download.
const clientDir = path.join(root, 'release', 'staging-client')
const appName = 'Google Foto Manager.app'
const outApp = path.join(clientDir, `GoogleFotoManager-${oldVersion}-test.app`)
const feedUrl = process.env.GFM_UPDATE_FEED_URL || 'http://127.0.0.1:8765/'

console.log(`Building test client ${oldVersion} (restore to ${currentVersion} after)`)
console.log(`Baking local feed: ${feedUrl}`)
pkg.version = oldVersion
fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`)

try {
  const compile = spawnSync('npm', ['run', 'build'], {
    cwd: root,
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, GFM_UPDATE_FEED_URL: feedUrl }
  })
  if ((compile.status ?? 1) !== 0) {
    process.exit(compile.status ?? 1)
  }

  const build = spawnSync(
    'npx',
    ['electron-builder', '--mac', '--arm64', '--config.mac.target=dir'],
    {
      cwd: root,
      stdio: 'inherit',
      shell: true,
      env: { ...process.env, GFM_UPDATE_FEED_URL: feedUrl }
    }
  )
  if ((build.status ?? 1) !== 0) {
    process.exit(build.status ?? 1)
  }

  const builtApp = path.join(root, 'release', 'mac-arm64', appName)
  const altApp = path.join(root, 'release', 'mac', appName)
  const source = fs.existsSync(builtApp) ? builtApp : altApp
  if (!fs.existsSync(source)) {
    console.error(`Built app not found under release/mac-arm64 or release/mac`)
    process.exit(1)
  }

  const ymlPath = path.join(source, 'Contents', 'Resources', 'app-update.yml')
  if (!fs.existsSync(ymlPath)) {
    console.error(`Missing app-update.yml after pack: ${ymlPath}`)
    process.exit(1)
  }
  const yml = fs.readFileSync(ymlPath, 'utf8')
  if (!yml.includes('provider: generic') || !yml.includes(feedUrl.replace(/\/$/, ''))) {
    console.error(`app-update.yml was not rewritten for local feed:\n${yml}`)
    process.exit(1)
  }

  fs.mkdirSync(clientDir, { recursive: true })
  fs.rmSync(outApp, { recursive: true, force: true })
  // ditto preserves macOS app bundles / signatures better than fs.cpSync
  const copy = spawnSync('ditto', [source, outApp], { encoding: 'utf8' })
  if ((copy.status ?? 1) !== 0) {
    console.error(copy.stderr || copy.stdout)
    process.exit(copy.status ?? 1)
  }
  console.log(`\nTest client: ${outApp}`)
  console.log(`Launch:\n  open "${outApp}"`)
  console.log('Keep npm run release:local-server running while testing.')
} finally {
  pkg.version = currentVersion
  fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`)
  console.log(`Restored package.json version → ${currentVersion}`)
}
