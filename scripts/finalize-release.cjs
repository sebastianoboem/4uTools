#!/usr/bin/env node
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')
const {
  githubAssetName,
  readVersion,
  releaseDir,
  userReleaseFiles
} = require('./rename-artifacts.cjs')

function sha512Base64(filePath) {
  return crypto.createHash('sha512').update(fs.readFileSync(filePath)).digest('base64')
}

function findExe(version) {
  const names = fs.readdirSync(releaseDir)
  const exe = names.find(
    (n) => n.endsWith('.exe') && n.includes(version) && !/-(x64|arm64)\.exe$/i.test(n)
  )
  if (!exe) throw new Error(`Windows installer not found in ${releaseDir}`)
  return path.join(releaseDir, exe)
}

function findMacZips(version) {
  const names = fs.readdirSync(releaseDir)
  const zips = names
    .filter((n) => n.endsWith('.zip') && n.includes(version))
    .sort()
  if (zips.length === 0) {
    throw new Error(`macOS zip artifacts not found in ${releaseDir}`)
  }
  return zips.map((name) => path.join(releaseDir, name))
}

function writeLatestYml(version, exePath) {
  const assetName = githubAssetName(path.basename(exePath))
  const sha = sha512Base64(exePath)
  const body = [
    `version: ${version}`,
    'files:',
    `  - url: ${assetName}`,
    `    sha512: ${sha}`,
    `path: ${assetName}`,
    `sha512: ${sha}`,
    `releaseDate: '${new Date().toISOString()}'`,
    ''
  ].join('\n')
  fs.writeFileSync(path.join(releaseDir, 'latest.yml'), body)
  console.log(`Wrote latest.yml → ${assetName}`)
}

function writeLatestMacYml(version, zipPaths) {
  const files = zipPaths.map((zipPath) => {
    const assetName = githubAssetName(path.basename(zipPath))
    const sha = sha512Base64(zipPath)
    return { assetName, sha, zipPath }
  })
  const primary = files[0]
  const lines = [
    `version: ${version}`,
    'files:'
  ]
  for (const file of files) {
    lines.push(`  - url: ${file.assetName}`)
    lines.push(`    sha512: ${file.sha}`)
  }
  lines.push(`path: ${primary.assetName}`)
  lines.push(`sha512: ${primary.sha}`)
  lines.push(`releaseDate: '${new Date().toISOString()}'`)
  lines.push('')
  fs.writeFileSync(path.join(releaseDir, 'latest-mac.yml'), lines.join('\n'))
  console.log(
    `Wrote latest-mac.yml → ${files.map((f) => f.assetName).join(', ')}`
  )
}

function cleanupReleaseDir(version) {
  let keepInstallers = []
  try {
    keepInstallers = userReleaseFiles(version).map((filePath) => path.basename(filePath))
  } catch {
    keepInstallers = fs
      .readdirSync(releaseDir)
      .filter(
        (name) =>
          (name.endsWith('.exe') && name.includes(version) && !/-(x64|arm64)\.exe$/i.test(name)) ||
          (name.endsWith('.dmg') && (name.includes('AppleSilicon') || name.includes('Intel'))) ||
          (name.endsWith('.zip') && name.includes(version))
      )
  }

  const keep = new Set([...keepInstallers, 'latest.yml', 'latest-mac.yml'])

  for (const name of fs.readdirSync(releaseDir)) {
    if (keep.has(name)) continue
    fs.rmSync(path.join(releaseDir, name), { recursive: true, force: true })
  }

  const remaining = fs.readdirSync(releaseDir).sort()
  console.log(`release/ contents: ${remaining.join(', ')}`)
  if (!remaining.includes('latest.yml')) {
    throw new Error('latest.yml missing after finalize')
  }
  if (!remaining.includes('latest-mac.yml')) {
    throw new Error('latest-mac.yml missing after finalize')
  }
}

const version = readVersion()
if (!fs.existsSync(releaseDir)) {
  console.error(`Missing ${releaseDir}`)
  process.exit(1)
}

const exePath = findExe(version)
const zipPaths = findMacZips(version)
writeLatestYml(version, exePath)
writeLatestMacYml(version, zipPaths)
cleanupReleaseDir(version)
