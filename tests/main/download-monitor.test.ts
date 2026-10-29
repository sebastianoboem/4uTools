import { describe, expect, it } from 'vitest'
import { DEFAULT_DOWNLOAD_WAIT_MS, downloadPercent } from '../../src/main/download-monitor'

describe('download wait', () => {
  it('waits 15 minutes for the download to start', () => {
    expect(DEFAULT_DOWNLOAD_WAIT_MS).toBe(15 * 60 * 1000)
  })
})

describe('downloadPercent', () => {
  it('returns 0 when total is unknown', () => {
    expect(downloadPercent(100, 0)).toBe(0)
    expect(downloadPercent(100, -1)).toBe(0)
  })

  it('computes clamped percentage from bytes', () => {
    expect(downloadPercent(0, 200)).toBe(0)
    expect(downloadPercent(50, 200)).toBe(25)
    expect(downloadPercent(200, 200)).toBe(100)
    expect(downloadPercent(250, 200)).toBe(100)
  })
})
