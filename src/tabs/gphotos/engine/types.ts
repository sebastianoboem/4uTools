export type EngineState =
  | 'idle'
  | 'running'
  | 'paused'
  | 'stopped'
  | 'downloading'
  | 'done'
  | 'error'

export interface SelectionParams {
  clickDelay: number
  scrollFraction: number
  settleDelay: number
  passesPerStep: number
  stallPasses: number
  skipLabelPrefix: string
  maxSelections: number
  downloadAfterSelection: boolean
}

export interface SavedProfile {
  id: string
  name: string
  createdAt: number
}

export interface ProfileStoreData {
  profiles: SavedProfile[]
  activeProfileId: string | null
}

export interface ProgressEvent {
  selected: number
  estPct: number
  elapsedMs: number
  state: EngineState
  /** Set when a partial selection is about to be deleted. */
  warning?: string
}

export type LogLevel = 'info' | 'azione' | 'avviso' | 'errore'

export interface LogEvent {
  ts: number
  level: LogLevel
  msg: string
}

export interface DoneEvent {
  total: number
  leftovers: number
}

export interface ErrorDiagnostics {
  scrollContainers: number
  mainElements: number
  checkboxes: number
  url?: string
}

export interface ErrorEvent {
  message: string
  diagnostics: ErrorDiagnostics
}

export interface PageStatus {
  isPhotosGoogle: boolean
  isLoginBlocked: boolean
  loginBlockMessage?: string
}
