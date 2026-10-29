import { describe, expect, it } from 'vitest'
import { getChromeClientHints, getCleanUserAgent } from '../../src/main/user-agent'

describe('user-agent', () => {
  it('removes Electron marker from UA string', () => {
    const dirty =
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Electron/36.3.1'
    const clean = getCleanUserAgent(dirty)
    expect(clean).not.toContain('Electron')
    expect(clean).toContain('Chrome/')
  })

  it('removes app name from UA string', () => {
    const dirty =
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 google-foto-manager/1.0.0'
    const clean = getCleanUserAgent(dirty)
    expect(clean).not.toContain('google-foto-manager')
  })

  it('provides Chrome-like client hints', () => {
    const hints = getChromeClientHints()
    expect(hints.secChUa).toContain('Google Chrome')
    expect(hints.secChUa).not.toContain('Electron')
    expect(hints.secChUaMobile).toBe('?0')
  })
})
