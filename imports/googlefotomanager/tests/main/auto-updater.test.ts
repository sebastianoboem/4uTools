import { describe, expect, it, vi } from 'vitest'

vi.mock('electron', () => ({
  app: { isPackaged: false },
  BrowserWindow: vi.fn(),
  dialog: { showMessageBox: vi.fn() }
}))

vi.mock('electron-updater', () => ({
  autoUpdater: {
    on: vi.fn(),
    setFeedURL: vi.fn(),
    checkForUpdates: vi.fn().mockResolvedValue(null),
    quitAndInstall: vi.fn()
  }
}))

describe('auto-updater', () => {
  it('skips update check in development', async () => {
    const { shouldRunStartupUpdateCheck, runStartupUpdateCheck } = await import(
      '../../src/main/auto-updater'
    )

    expect(shouldRunStartupUpdateCheck()).toBe(false)
    await expect(runStartupUpdateCheck()).resolves.toBeUndefined()
  })
})
