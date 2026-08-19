#!/usr/bin/env node
/**
 * For local updater staging, write a generic app-update.yml before codesign.
 * Production builds keep electron-builder's GitHub app-update.yml.
 */
const fs = require('node:fs')
const path = require('node:path')

exports.default = async function afterPack(context) {
  const feed = process.env.GFM_UPDATE_FEED_URL?.trim()
  if (!feed) return
  if (context.electronPlatformName !== 'darwin') return

  const appName = `${context.packager.appInfo.productFilename}.app`
  const ymlPath = path.join(
    context.appOutDir,
    appName,
    'Contents',
    'Resources',
    'app-update.yml'
  )
  const url = feed.endsWith('/') ? feed : `${feed}/`
  fs.mkdirSync(path.dirname(ymlPath), { recursive: true })
  fs.writeFileSync(
    ymlPath,
    [
      'provider: generic',
      `url: ${url}`,
      'updaterCacheDirName: google-foto-manager-updater',
      ''
    ].join('\n')
  )
  console.log(`afterPack: wrote local app-update.yml → ${url}`)
}
