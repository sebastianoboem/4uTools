import { afterEach, describe, expect, it } from 'vitest'
import {
  ZOOM_DEFAULT,
  ZOOM_MIN,
  applyZoom,
  clampZoom,
  getZoomState,
  resetLastZoomFactor,
  zoomToPercent
} from '../../src/main/zoom'

describe('zoom', () => {
  afterEach(() => {
    resetLastZoomFactor()
  })

  it('uses 30% as the minimum and 50% as the default', () => {
    expect(ZOOM_MIN).toBe(0.3)
    expect(ZOOM_DEFAULT).toBe(0.5)
    expect(zoomToPercent(ZOOM_DEFAULT)).toBe(50)
  })

  it('clamps below 30% up to the minimum', () => {
    expect(clampZoom(0.2)).toBe(0.3)
    expect(zoomToPercent(0.2)).toBe(30)
  })

  it('reports last applied zoom even if webContents still returns 1', () => {
    const webContents = {
      setZoomFactor: () => {},
      getZoomFactor: () => 1
    }
    applyZoom(webContents as never, 0.5)
    expect(getZoomState(webContents as never).percent).toBe(50)
  })
})
