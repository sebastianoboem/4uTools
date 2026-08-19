import type { WebContents } from 'electron'

export const ZOOM_MIN = 0.3
export const ZOOM_MAX = 3.0
export const ZOOM_STEP = 0.1
export const ZOOM_DEFAULT = 0.5

export interface ZoomState {
  factor: number
  percent: number
}

let lastZoomFactor: number | null = null

/** Test helper: clear remembered zoom so cases stay isolated. */
export function resetLastZoomFactor(): void {
  lastZoomFactor = null
}

export function clampZoom(factor: number): number {
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, factor))
}

export function zoomToPercent(factor: number): number {
  return Math.round(clampZoom(factor) * 100)
}

export function getZoomState(webContents: WebContents | null): ZoomState {
  const factor =
    lastZoomFactor ?? (webContents ? webContents.getZoomFactor() : ZOOM_DEFAULT)
  return { factor, percent: zoomToPercent(factor) }
}

export function applyZoom(webContents: WebContents, factor: number): ZoomState {
  const next = clampZoom(factor)
  lastZoomFactor = next
  webContents.setZoomFactor(next)
  return { factor: next, percent: zoomToPercent(next) }
}

export function stepZoom(webContents: WebContents, direction: -1 | 1): ZoomState {
  const current = lastZoomFactor ?? webContents.getZoomFactor()
  return applyZoom(webContents, current + direction * ZOOM_STEP)
}

export function resetZoom(webContents: WebContents): ZoomState {
  return applyZoom(webContents, ZOOM_DEFAULT)
}
