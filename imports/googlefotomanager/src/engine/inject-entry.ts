import type { DoneEvent, ErrorEvent, LogEvent, ProgressEvent } from '../shared/types'
import { engine } from './selection-engine'

declare global {
  interface Window {
    __googleFotoEngine?: typeof engine
    __gfmEmitProgress?: (e: ProgressEvent) => void
    __gfmEmitLog?: (e: LogEvent) => void
    __gfmEmitDone?: (e: DoneEvent) => void
    __gfmEmitError?: (e: ErrorEvent) => void
  }
}

engine.setCallbacks({
  onProgress: (e) => window.__gfmEmitProgress?.(e),
  onLog: (e) => window.__gfmEmitLog?.(e),
  onDone: (e) => window.__gfmEmitDone?.(e),
  onError: (e) => window.__gfmEmitError?.(e)
})

window.__googleFotoEngine = engine
