import { describe, expect, it } from 'vitest'
import {
  buildChromeLoginLaunchOptions,
  buildLoginLaunchOptions,
  getEdgeExecutableCandidates
} from '../../src/main/chrome-login-bridge'

describe('buildLoginLaunchOptions', () => {
  it('disables Puppeteer automation flags for Chrome sign-in', () => {
    const opts = buildLoginLaunchOptions('/tmp/gfm-login-xyz', {
      kind: 'chrome',
      displayName: 'Chrome'
    })
    expect(opts.ignoreDefaultArgs).toContain('--enable-automation')
    expect(opts.args).toContain('--disable-blink-features=AutomationControlled')
    expect(opts.channel).toBe('chrome')
    expect(opts.headless).toBe(false)
    expect(opts.userDataDir).toBe('/tmp/gfm-login-xyz')
  })

  it('uses executablePath for Edge fallback', () => {
    const edgePath = 'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe'
    const opts = buildLoginLaunchOptions('/tmp/gfm-login-edge', {
      kind: 'edge',
      displayName: 'Microsoft Edge',
      executablePath: edgePath
    })
    expect(opts.executablePath).toBe(edgePath)
    expect(opts.channel).toBeUndefined()
    expect(opts.ignoreDefaultArgs).toContain('--enable-automation')
  })

  it('buildChromeLoginLaunchOptions remains Chrome-compatible', () => {
    const opts = buildChromeLoginLaunchOptions('/tmp/gfm-login-xyz')
    expect(opts.channel).toBe('chrome')
  })
})

describe('getEdgeExecutableCandidates', () => {
  it('includes standard Windows Edge paths', () => {
    const original = process.platform
    Object.defineProperty(process, 'platform', { configurable: true, value: 'win32' })
    const paths = getEdgeExecutableCandidates()
    Object.defineProperty(process, 'platform', { configurable: true, value: original })

    expect(paths.some((p) => p.includes('msedge.exe'))).toBe(true)
  })
})
