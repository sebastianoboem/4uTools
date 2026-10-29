import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DEFAULT_SELECTION_PARAMS } from '../../src/shared/settings-defaults'

const mockUserData = '/tmp/google-foto-test-userdata'

vi.mock('electron', () => ({
  app: {
    getPath: vi.fn(() => mockUserData)
  }
}))

describe('settings-store', () => {
  beforeEach(async () => {
    vi.resetModules()
    const { mkdirSync, rmSync } = await import('node:fs')
    rmSync(mockUserData, { recursive: true, force: true })
    mkdirSync(mockUserData, { recursive: true })
  })

  afterEach(async () => {
    const { rmSync } = await import('node:fs')
    rmSync(mockUserData, { recursive: true, force: true })
  })

  it('round-trips settings read/write/reset', async () => {
    const store = await import('../../src/main/settings-store')

    const defaults = store.loadSettings()
    expect(defaults.clickDelay).toBe(DEFAULT_SELECTION_PARAMS.clickDelay)

    const saved = store.saveSettings({
      ...DEFAULT_SELECTION_PARAMS,
      clickDelay: 120,
      scrollFraction: 60
    })
    expect(saved.clickDelay).toBe(120)

    const loaded = store.loadSettings()
    expect(loaded.clickDelay).toBe(120)
    expect(loaded.scrollFraction).toBe(60)

    const reset = store.resetSettings()
    expect(reset.clickDelay).toBe(DEFAULT_SELECTION_PARAMS.clickDelay)
  })
})
