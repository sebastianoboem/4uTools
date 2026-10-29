/// <reference types="electron-vite/node" />

import type { SelectionEngine } from './engine/selection-engine'

declare global {
  interface Window {
    __googleFotoEngine?: SelectionEngine
    __gfmEmitProgress?: (e: import('./shared/types').ProgressEvent) => void
    __gfmEmitLog?: (e: import('./shared/types').LogEvent) => void
    __gfmEmitDone?: (e: import('./shared/types').DoneEvent) => void
    __gfmEmitError?: (e: import('./shared/types').ErrorEvent) => void
  }
}

export {}
