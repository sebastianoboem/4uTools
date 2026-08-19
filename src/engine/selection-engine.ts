import type {
  DoneEvent,
  EngineState,
  ErrorEvent,
  LogEvent,
  ProgressEvent,
  SelectionParams
} from '../shared/types'
import { DEFAULT_SELECTION_PARAMS } from '../shared/settings-defaults'
import {
  detectLoginBlock,
  findClearSelectionButton,
  findDeleteButton,
  findDeleteConfirmButton,
  findDeleteMenuItem,
  findDownloadMenuItem,
  findMainElement,
  findMoreOptionsButton,
  findScrollContainer,
  gatherDiagnostics,
  isPhotosGoogleUrl,
  isSkipLabel,
  readGooglePhotosSelectionCount
} from './selectors'

export type EngineCallbacks = {
  onProgress?: (event: ProgressEvent) => void
  onLog?: (event: LogEvent) => void
  onDone?: (event: DoneEvent) => void
  onError?: (event: ErrorEvent) => void
  onTriggerDownload?: () => Promise<{ ok: boolean; method?: string; step?: string }>
}

const delay = (ms: number): Promise<void> => new Promise((res) => setTimeout(res, ms))
const PROGRESS_TICK_MS = 250

export class SelectionEngine {
  private params: SelectionParams = { ...DEFAULT_SELECTION_PARAMS }
  private state: EngineState = 'idle'
  private controlState: 'running' | 'paused' | 'stopped' = 'stopped'
  private totalSelected = 0
  private estPct = 0
  private startedAt = 0
  private accumulatedPauseMs = 0
  private pausedAt = 0
  private progressTicker: ReturnType<typeof setInterval> | null = null
  private activeScrollEl: HTMLElement | null = null
  private loopPromise: Promise<void> | null = null
  private callbacks: EngineCallbacks = {}
  private shouldDownload = false
  private shouldDelete = false

  setCallbacks(callbacks: EngineCallbacks): void {
    this.callbacks = callbacks
  }

  getState(): EngineState {
    return this.state
  }

  getSelectedCount(): number {
    return this.getReportedSelectedCount()
  }

  private getReportedSelectedCount(): number {
    return readGooglePhotosSelectionCount() ?? this.totalSelected
  }

  diagnose() {
    return gatherDiagnostics(document, window.location.href)
  }

  getPageStatus() {
    const url = window.location.href
    const login = detectLoginBlock()
    return {
      isPhotosGoogle: isPhotosGoogleUrl(url),
      isLoginBlocked: login.blocked,
      loginBlockMessage: login.message
    }
  }

  async start(params?: Partial<SelectionParams>): Promise<void> {
    if (this.controlState === 'running') return
    this.params = { ...DEFAULT_SELECTION_PARAMS, ...params }
    this.shouldDownload = this.params.downloadAfterSelection
    this.shouldDelete = false
    await this.runSelection()
  }

  async startAndDelete(params?: Partial<SelectionParams>): Promise<void> {
    if (this.controlState === 'running') return
    this.params = { ...DEFAULT_SELECTION_PARAMS, ...params }
    this.shouldDownload = this.params.downloadAfterSelection
    this.shouldDelete = true
    await this.runSelection()
  }

  private async runSelection(): Promise<void> {
    this.totalSelected = 0
    this.estPct = 0
    this.startedAt = Date.now()
    this.accumulatedPauseMs = 0
    this.pausedAt = 0
    this.controlState = 'running'
    this.state = 'running'
    this.emitLog('azione', 'Avvio selezione foto')
    this.emitProgress()

    const scrollEl = findScrollContainer()
    const mainEl = findMainElement()

    if (!mainEl) {
      this.fail('Selettori DOM non trovati', gatherDiagnostics(document, window.location.href))
      return
    }

    if (!scrollEl) {
      await this.runWithoutScrollContainer(mainEl)
      return
    }

    this.startProgressTicker(scrollEl)
    this.loopPromise = this.runLoop(scrollEl, mainEl)
    try {
      await this.loopPromise
    } finally {
      this.stopProgressTicker()
    }
  }

  private async runWithoutScrollContainer(mainEl: HTMLElement): Promise<void> {
    const existingSelection = this.getReportedSelectedCount()

    if (this.shouldDelete && existingSelection > 0) {
      this.emitLog('info', `${existingSelection} foto già selezionate — avvio eliminazione`)
      await this.completePostAction(existingSelection, 0)
      return
    }

    this.emitLog('avviso', 'Contenitore scroll non rilevato — selezione limitata al viewport visibile')

    for (let pass = 0; pass < this.params.passesPerStep; pass++) {
      if (this.controlState === 'stopped') return
      await this.waitIfPaused()
      await this.selectVisible(mainEl)
      if (pass + 1 < this.params.passesPerStep) await delay(this.params.settleDelay)
    }

    if (this.controlState === 'stopped') return

    const leftovers = await this.verifyLeftovers(mainEl)
    const total = this.getReportedSelectedCount()
    if (total <= 0) {
      this.fail('Nessuna foto selezionabile nel viewport', gatherDiagnostics(document, window.location.href))
      return
    }

    await this.completePostAction(total, leftovers)
  }

  private async completePostAction(total: number, leftovers: number): Promise<void> {
    let downloadOk = true
    if (this.shouldDownload && total > 0) {
      downloadOk = await this.triggerDownload()
    }
    if (this.shouldDelete && total > 0) {
      if (this.shouldDownload && !downloadOk) {
        this.emitLog('avviso', 'Eliminazione saltata perché il download non è riuscito')
      } else {
        await this.triggerDelete()
      }
    }

    this.shouldDownload = false
    this.shouldDelete = false
    this.state = 'done'
    this.controlState = 'stopped'
    this.emitProgress()
    this.callbacks.onDone?.({ total, leftovers })
  }

  pause(): void {
    if (this.controlState !== 'running') return
    this.controlState = 'paused'
    this.state = 'paused'
    this.pausedAt = Date.now()
    this.stopProgressTicker()
    this.emitLog('info', 'Selezione in pausa')
    this.emitProgress()
  }

  resume(): void {
    if (this.controlState !== 'paused') return
    if (this.pausedAt > 0) {
      this.accumulatedPauseMs += Date.now() - this.pausedAt
      this.pausedAt = 0
    }
    this.controlState = 'running'
    this.state = 'running'
    if (this.activeScrollEl) this.startProgressTicker(this.activeScrollEl)
    this.emitLog('azione', 'Selezione ripresa')
    this.emitProgress()
  }

  async stop(): Promise<void> {
    this.stopProgressTicker()
    const wasActive =
      this.controlState === 'running' ||
      this.controlState === 'paused' ||
      this.state === 'running' ||
      this.state === 'paused'

    if (wasActive) {
      this.controlState = 'stopped'
      this.state = 'stopped'
      this.emitLog('avviso', 'Selezione interrotta')
    }

    if (this.loopPromise) {
      await this.loopPromise.catch(() => {})
      this.loopPromise = null
    }

    await this.clickClearSelection()
    this.scrollToTop()
    this.resetRunState()
  }

  private scrollToTop(): void {
    const scrollEl = findScrollContainer()
    if (!scrollEl) return
    if (typeof scrollEl.scrollTo === 'function') {
      scrollEl.scrollTo({ top: 0, behavior: 'auto' })
    } else {
      scrollEl.scrollTop = 0
    }
    this.emitLog('info', 'Scroll riportato all\'inizio')
  }

  async deselect(): Promise<void> {
    if (this.controlState === 'running' || this.controlState === 'paused') {
      await this.stop()
      return
    }
    await this.clickClearSelection()
    this.resetRunState()
  }

  private async clickClearSelection(): Promise<void> {
    const clearBtn = findClearSelectionButton()
    if (clearBtn) {
      clearBtn.click()
      this.emitLog('azione', 'Deselezione tramite «Cancella selezione»')
      return
    }

    const mainEl = findMainElement()
    if (!mainEl) {
      this.emitLog('avviso', 'Pulsante «Cancella selezione» non trovato')
      return
    }

    const checked = mainEl.querySelectorAll('div[role="checkbox"][aria-checked="true"]')
    for (const cb of checked) {
      ;(cb as HTMLElement).click()
      await delay(this.params.clickDelay)
    }
    if (checked.length > 0) {
      this.emitLog('azione', `Deselezionate ${checked.length} checkbox nel viewport`)
    }
  }

  private resetRunState(): void {
    this.stopProgressTicker()
    this.totalSelected = 0
    this.estPct = 0
    this.startedAt = 0
    this.accumulatedPauseMs = 0
    this.pausedAt = 0
    this.state = 'idle'
    this.controlState = 'stopped'
    this.emitResetProgress()
  }

  private startProgressTicker(scrollEl: HTMLElement): void {
    this.stopProgressTicker()
    this.activeScrollEl = scrollEl
    this.progressTicker = setInterval(() => {
      if (this.controlState !== 'running') return
      this.updateEstPct(scrollEl)
      this.emitProgress()
    }, PROGRESS_TICK_MS)
  }

  private stopProgressTicker(): void {
    if (this.progressTicker) {
      clearInterval(this.progressTicker)
      this.progressTicker = null
    }
    this.activeScrollEl = null
  }

  private getElapsedMs(): number {
    if (!this.startedAt) return 0
    const now = Date.now()
    const currentPauseMs =
      this.controlState === 'paused' && this.pausedAt > 0 ? now - this.pausedAt : 0
    return Math.max(0, now - this.startedAt - this.accumulatedPauseMs - currentPauseMs)
  }

  private emitResetProgress(): void {
    this.callbacks.onProgress?.({
      selected: 0,
      estPct: 0,
      elapsedMs: 0,
      state: 'idle'
    })
  }

  private async runLoop(scrollEl: HTMLElement, mainEl: HTMLElement): Promise<void> {
    let lastScrollHeight = 0
    let noChangeCount = 0

    while (this.controlState !== 'stopped') {
      await this.waitIfPaused()
      if (this.controlState === 'stopped') break

      for (let pass = 0; pass < this.params.passesPerStep; pass++) {
        await this.selectVisible(mainEl)
        if (this.controlState === 'stopped') break
        await delay(this.params.clickDelay)
      }

      if (this.controlState === 'stopped') break

      const step = scrollEl.clientHeight * (this.params.scrollFraction / 100)
      if (typeof scrollEl.scrollBy === 'function') {
        scrollEl.scrollBy({ top: step, behavior: 'auto' })
      } else {
        scrollEl.scrollTop = Math.min(
          scrollEl.scrollTop + step,
          scrollEl.scrollHeight - scrollEl.clientHeight
        )
      }
      await delay(this.params.settleDelay)

      for (let pass = 0; pass < this.params.passesPerStep; pass++) {
        await this.selectVisible(mainEl)
        if (this.controlState === 'stopped') break
        await delay(this.params.clickDelay)
      }

      this.updateEstPct(scrollEl)
      this.emitProgress()

      const currentScrollHeight = scrollEl.scrollHeight
      if (currentScrollHeight === lastScrollHeight) {
        noChangeCount++
        const atBottom =
          scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 100
        if (noChangeCount >= this.params.stallPasses && atBottom) {
          break
        }
      } else {
        noChangeCount = 0
        lastScrollHeight = currentScrollHeight
      }

      if (
        this.params.maxSelections > 0 &&
        this.totalSelected >= this.params.maxSelections
      ) {
        this.emitLog('avviso', `Raggiunto limite di ${this.params.maxSelections} selezioni`)
        break
      }
    }

    if (this.controlState === 'stopped') return

    const leftovers = await this.verifyLeftovers(mainEl)
    const total = this.getReportedSelectedCount()
    this.emitLog('info', `Completato: ${total} foto selezionate (Google Foto)`)
    await this.completePostAction(total, leftovers)
  }

  private async triggerDownload(): Promise<boolean> {
    this.emitLog('azione', 'Avvio download…')
    this.stopProgressTicker()
    this.state = 'downloading'
    this.estPct = 0
    this.emitProgress()

    if (this.callbacks.onTriggerDownload) {
      const result = await this.callbacks.onTriggerDownload()
      if (result.ok) {
        this.estPct = 100
        this.emitLog(
          'info',
          result.method === 'menu'
            ? 'Download completato (menu)'
            : result.method === 'shortcut'
              ? 'Download completato (Maiusc+D)'
              : 'Download completato'
        )
        return true
      }
      this.emitLog('errore', `Download non riuscito (${result.step ?? 'sconosciuto'})`)
      return false
    }

    const menuBtn = findMoreOptionsButton()
    if (menuBtn) {
      menuBtn.click()
      await delay(300)
      const downloadItem = findDownloadMenuItem()
      if (downloadItem) {
        downloadItem.click()
        this.emitLog('info', 'Download avviato dal menu')
        return true
      }
    }

    this.emitLog('errore', 'Impossibile avviare il download')
    return false
  }

  private async triggerDelete(): Promise<void> {
    this.emitLog('azione', 'Eliminazione foto selezionate…')
    const deleteBtn = findDeleteButton()
    if (!deleteBtn) {
      this.emitLog('errore', 'Pulsante cestino non trovato nella barra strumenti')
      return
    }
    deleteBtn.click()

    let steps = 0
    for (let attempt = 0; attempt < 25; attempt++) {
      await delay(200)
      const dialogCount = document.querySelectorAll('[role="dialog"], [role="alertdialog"]').length

      const menuItem = findDeleteMenuItem()
      if (menuItem) {
        menuItem.click()
        steps++
        continue
      }

      const confirmBtn = findDeleteConfirmButton()
      if (confirmBtn) {
        confirmBtn.click()
        steps++
        continue
      }

      if (steps > 0 && dialogCount === 0) {
        this.emitLog('info', 'Eliminazione confermata')
        return
      }
    }

    const remainingDialogs = document.querySelectorAll('[role="dialog"], [role="alertdialog"]').length
    if (steps > 0 && remainingDialogs === 0) {
      this.emitLog('info', 'Eliminazione confermata')
      return
    }

    this.emitLog('avviso', 'Finestra di conferma non trovata — verifica manualmente in Google Foto')
  }

  private async selectVisible(mainEl: HTMLElement): Promise<number> {
    const checkboxes = mainEl.querySelectorAll<HTMLElement>('div[role="checkbox"]')
    let newSelected = 0

    for (const cb of checkboxes) {
      if (this.controlState === 'stopped') break
      await this.waitIfPaused()
      if (this.controlState === 'stopped') break

      const label = cb.getAttribute('aria-label') || ''
      const alreadyChecked = cb.getAttribute('aria-checked') === 'true'

      if (!alreadyChecked && !isSkipLabel(label, this.params.skipLabelPrefix)) {
        cb.click()
        newSelected++
        this.totalSelected++
        await delay(this.params.clickDelay)
      }
    }

    if (newSelected > 0) {
      const total = this.getReportedSelectedCount()
      this.emitLog('info', `+${newSelected} nel viewport (totale Google Foto: ${total})`)
    }

    return newSelected
  }

  private async verifyLeftovers(mainEl: HTMLElement): Promise<number> {
    let leftovers = 0
    const checkboxes = mainEl.querySelectorAll<HTMLElement>('div[role="checkbox"]')
    for (const cb of checkboxes) {
      const label = cb.getAttribute('aria-label') || ''
      if (
        cb.getAttribute('aria-checked') !== 'true' &&
        !isSkipLabel(label, this.params.skipLabelPrefix)
      ) {
        leftovers++
      }
    }
    if (leftovers > 0) {
      this.emitLog('avviso', `${leftovers} checkbox non selezionate nel viewport finale`)
    }
    return leftovers
  }

  private updateEstPct(scrollEl: HTMLElement): void {
    const maxScroll = scrollEl.scrollHeight - scrollEl.clientHeight
    if (maxScroll <= 0) {
      this.estPct = 100
      return
    }
    const raw = (scrollEl.scrollTop / maxScroll) * 100
    this.estPct = Math.max(this.estPct, Math.min(100, raw))
  }

  private async waitIfPaused(): Promise<void> {
    while (this.controlState === 'paused') {
      await delay(100)
    }
  }

  private emitProgress(): void {
    this.callbacks.onProgress?.({
      selected: this.getReportedSelectedCount(),
      estPct: Math.min(100, Math.max(0, this.estPct)),
      elapsedMs: this.getElapsedMs(),
      state: this.state
    })
  }

  private emitLog(level: LogEvent['level'], msg: string): void {
    this.callbacks.onLog?.({ ts: Date.now(), level, msg })
  }

  private fail(message: string, diagnostics: ErrorEvent['diagnostics']): void {
    this.stopProgressTicker()
    this.state = 'error'
    this.controlState = 'stopped'
    this.emitLog('errore', message)
    this.emitProgress()
    this.callbacks.onError?.({ message, diagnostics })
  }
}

export const engine = new SelectionEngine()
