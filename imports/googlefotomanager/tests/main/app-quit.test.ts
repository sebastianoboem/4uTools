import { describe, expect, it, vi } from 'vitest'
import { prepareAppQuit, setPrepareAppQuit } from '../../src/main/app-quit'

describe('app-quit', () => {
  it('runs registered quit preparer', () => {
    const fn = vi.fn()
    setPrepareAppQuit(fn)
    prepareAppQuit()
    expect(fn).toHaveBeenCalledOnce()
  })
})
