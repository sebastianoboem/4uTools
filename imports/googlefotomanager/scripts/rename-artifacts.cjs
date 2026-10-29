const fs = require('node:fs')
const path = require('node:path')

const releaseDir = path.join(__dirname, '..', 'release')
const MAC_RENAMES = new Map([
  ['-arm64.dmg', '-arm64-AppleSilicon.dmg'],
  ['-x64.dmg', '-x64-Intel.dmg']
])

function readVersion() {
  const pkg = JSON.parse(
    fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8')
  )
  return pkg.version
}

function githubAssetName(localName) {
  return localName.replace(/ /g, '.')
}

function userReleaseFiles(version, dir = releaseDir) {
  const names = fs.readdirSync(dir)
  const exe = names.find(
    (n) => n.endsWith('.exe') && n.includes(version) && !/-(x64|arm64)\.exe$/i.test(n)
  )
  const dmgArm = names.find((n) => n.endsWith('.dmg') && n.includes('AppleSilicon'))
  const dmgIntel = names.find((n) => n.endsWith('.dmg') && n.includes('Intel'))
  const zipArm = names.find(
    (n) => n.endsWith('.zip') && n.includes(version) && n.includes('arm64')
  )
  const zipIntel = names.find(
    (n) => n.endsWith('.zip') && n.includes(version) && n.includes('x64')
  )
  if (!exe || !dmgArm || !dmgIntel || !zipArm || !zipIntel) {
    throw new Error(
      `Expected exe + 2 dmg + 2 zip in ${dir}, found: ${names.join(', ')}`
    )
  }
  return [exe, dmgArm, dmgIntel, zipArm, zipIntel].map((name) => path.join(dir, name))
}

function updateMetadataFiles(dir = releaseDir) {
  return ['latest.yml', 'latest-mac.yml']
    .map((name) => path.join(dir, name))
    .filter((filePath) => fs.existsSync(filePath))
}

function publishReleaseFiles(version, dir = releaseDir) {
  return [...userReleaseFiles(version, dir), ...updateMetadataFiles(dir)]
}

/** @param {import('app-builder-lib').BuildResult} buildResult */
module.exports = async function renameArtifacts(buildResult) {
  const updated = []

  for (const artifact of buildResult.artifactPaths) {
    let current = artifact

    for (const [from, to] of MAC_RENAMES) {
      if (current.endsWith(from)) {
        const dest = current.slice(0, -from.length) + to
        fs.renameSync(current, dest)
        current = dest
        break
      }
    }

    if (/-(x64|arm64)\.exe$/i.test(current) && fs.existsSync(current)) {
      fs.unlinkSync(current)
      continue
    }

    updated.push(current)
  }

  return updated
}

module.exports.userReleaseFiles = userReleaseFiles
module.exports.publishReleaseFiles = publishReleaseFiles
module.exports.updateMetadataFiles = updateMetadataFiles
module.exports.githubAssetName = githubAssetName
module.exports.readVersion = readVersion
module.exports.releaseDir = releaseDir
