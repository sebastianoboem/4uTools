import type {
  DoneEvent,
  EngineState,
  ErrorEvent,
  LogEvent,
  ProgressEvent,
  SelectionParams
} from '../shared/types'
import { DEFAULT_SELECTION_PARAMS, PARAM_RANGES } from '../shared/settings-defaults'
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
  private failedChecks = new Set<HTMLElement>()
  private failedLabel = new WeakMap<HTMLElement, string>()

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
    this.failedChecks = new Set()
    this.failedLabel = new WeakMap()
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
        if (leftovers > 0) {
          const warning = `${leftovers} checkbox non selezionate. Elimino comunque le altre.`
          this.emitLog('avviso', warning)
          this.emitProgress(warning)
        }
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
    this.failedChecks = new Set()
    this.failedLabel = new WeakMap()
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

      const beforePreviews = this.snapshotPreviews(mainEl)
      this.scrollByStep(scrollEl, mainEl)
      await this.waitForPreviews(mainEl, beforePreviews)

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
      if (this.failedChecks.has(cb) && this.failedLabel.get(cb) !== label) {
        this.failedChecks.delete(cb)
      }

      if (
        !alreadyChecked &&
        !isSkipLabel(label, this.params.skipLabelPrefix) &&
        !this.failedChecks.has(cb)
      ) {
        if (await this.clickCheckbox(cb)) {
          newSelected++
          this.totalSelected++
        }
        await delay(this.params.clickDelay)
      }
    }

    if (newSelected > 0) {
      const total = this.getReportedSelectedCount()
      this.emitLog('info', `+${newSelected} nel viewport (totale Google Foto: ${total})`)
    }

    return newSelected
  }

  private async clickCheckbox(cb: HTMLElement): Promise<boolean> {
    const label = cb.getAttribute('aria-label') || ''
    cb.click()
    if (await this.selectionLanded(cb, label)) return true
    if (this.tileChanged(cb, label)) return false
    this.retryClick(cb)
    if (await this.selectionLanded(cb, label)) return true
    if (!this.tileChanged(cb, label)) {
      this.failedChecks.add(cb)
      this.failedLabel.set(cb, label)
    }
    return false
  }

  private tileChanged(cb: HTMLElement, label: string): boolean {
    return (cb.getAttribute('aria-label') || '') !== label
  }

  /** Google Foto updates aria-checked after the click handler. A second click in the same turn undoes the first. */
  private async selectionLanded(cb: HTMLElement, label: string): Promise<boolean> {
    if (cb.getAttribute('aria-checked') === 'true') {
      this.failedChecks.delete(cb)
      return true
    }
    await delay(Math.max(this.params.clickDelay, 16))
    if (this.tileChanged(cb, label)) return false
    if (cb.getAttribute('aria-checked') !== 'true') return false
    this.failedChecks.delete(cb)
    return true
  }

  private retryClick(cb: HTMLElement): void {
    const view = cb.ownerDocument.defaultView
    if (view && typeof view.PointerEvent === 'function') {
      cb.dispatchEvent(new view.PointerEvent('pointerdown', { bubbles: true }))
      cb.dispatchEvent(new view.PointerEvent('pointerup', { bubbles: true }))
    }
    cb.dispatchEvent(new MouseEvent('click', { bubbles: true }))
  }

  private async verifyLeftovers(mainEl: HTMLElement): Promise<number> {
    const unchecked = (cb: HTMLElement): boolean => {
      const label = cb.getAttribute('aria-label') || ''
      return cb.getAttribute('aria-checked') !== 'true' && !isSkipLabel(label, this.params.skipLabelPrefix)
    }

    const failed = [...this.failedChecks].filter((cb) => {
      if (!unchecked(cb)) return false
      const remembered = this.failedLabel.get(cb)
      if (remembered === undefined) return true
      return remembered === (cb.getAttribute('aria-label') || '')
    })
    let leftovers = failed.length
    const failedSet = new Set(failed)
    for (const cb of mainEl.querySelectorAll<HTMLElement>('div[role="checkbox"]')) {
      if (unchecked(cb) && !failedSet.has(cb)) leftovers++
    }
    if (leftovers > 0) {
      this.emitLog('avviso', `${leftovers} checkbox non selezionate`)
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

  private scrollByStep(scrollEl: HTMLElement, mainEl: HTMLElement): void {
    const step = this.scrollStep(scrollEl, mainEl)
    if (typeof scrollEl.scrollBy === 'function') {
      scrollEl.scrollBy({ top: step, behavior: 'auto' })
    } else {
      scrollEl.scrollTop = Math.min(
        scrollEl.scrollTop + step,
        scrollEl.scrollHeight - scrollEl.clientHeight
      )
    }
  }

  private scrollStep(scrollEl: HTMLElement, mainEl: HTMLElement): number {
    const fraction =
      this.params.scrollFraction > PARAM_RANGES.scrollFraction.max
        ? DEFAULT_SELECTION_PARAMS.scrollFraction
        : this.params.scrollFraction
    const safe = this.safeScrollDistance(scrollEl, mainEl)
    if (safe > 0) return Math.max(1, (safe * fraction) / 100)
    const client = scrollEl.clientHeight
    if (client <= 0) return 0
    return (client * fraction) / 100
  }

  /** Largest scroll that keeps the lowest mounted checkbox inside the viewport. */
  private safeScrollDistance(scrollEl: HTMLElement, mainEl: HTMLElement): number {
    const viewTop = scrollEl.getBoundingClientRect().top
    const margin = 24
    let lowestBottom = Number.NEGATIVE_INFINITY
    for (const box of mainEl.querySelectorAll<HTMLElement>('div[role="checkbox"]')) {
      const rect = box.getBoundingClientRect()
      if (rect.width <= 0 || rect.height <= 0) continue
      lowestBottom = Math.max(lowestBottom, rect.bottom)
    }
    if (!Number.isFinite(lowestBottom)) return 0
    const distance = lowestBottom - viewTop - margin
    return distance > 1 ? distance : 0
  }

  private snapshotPreviews(mainEl: HTMLElement): Map<HTMLImageElement, string> {
    const shot = new Map<HTMLImageElement, string>()
    for (const img of mainEl.querySelectorAll('img')) shot.set(img, this.previewSrc(img))
    return shot
  }

  private freshPreviews(
    mainEl: HTMLElement,
    before: Map<HTMLImageElement, string>
  ): HTMLImageElement[] {
    const fresh: HTMLImageElement[] = []
    for (const img of mainEl.querySelectorAll('img')) {
      const src = this.previewSrc(img)
      const prev = before.get(img)
      if (prev === undefined || prev !== src) fresh.push(img)
    }
    return fresh
  }

  private previewSrc(img: HTMLImageElement): string {
    return img.currentSrc || img.getAttribute('src') || img.src || ''
  }

  private previewSettled(img: HTMLImageElement): boolean {
    return img.dataset.gfmPreviewError === '1' || (img.complete && img.naturalWidth > 0)
  }

  private async waitForPreviews(
    mainEl: HTMLElement,
    before: Map<HTMLImageElement, string>
  ): Promise<void> {
    // shortcut: imgs mounted after this turn are not waited for; upgrade if Google inserts them on a later frame
    const deadline = Date.now() + this.params.settleDelay
    while (this.controlState !== 'stopped') {
      await this.waitIfPaused()
      if (this.controlState === 'stopped') return
      const fresh = this.freshPreviews(mainEl, before)
      if (fresh.length === 0 || fresh.every((img) => this.previewSettled(img))) return
      if (Date.now() >= deadline) return
      await this.nextPreviewBeat(mainEl, Math.min(50, Math.max(0, deadline - Date.now())))
    }
  }

  private nextPreviewBeat(mainEl: HTMLElement, ms: number): Promise<void> {
    return new Promise((resolve) => {
      const timer = setTimeout(finish, ms)
      const onLoad = (): void => finish()
      const onError = (ev: Event): void => {
        const target = ev.target
        if (target instanceof HTMLImageElement) target.dataset.gfmPreviewError = '1'
        finish()
      }
      function finish(): void {
        clearTimeout(timer)
        mainEl.removeEventListener('load', onLoad, true)
        mainEl.removeEventListener('error', onError, true)
        resolve()
      }
      mainEl.addEventListener('load', onLoad, true)
      mainEl.addEventListener('error', onError, true)
    })
  }

  private emitProgress(warning?: string): void {
    this.callbacks.onProgress?.({
      selected: this.getReportedSelectedCount(),
      estPct: Math.min(100, Math.max(0, this.estPct)),
      elapsedMs: this.getElapsedMs(),
      state: this.state,
      ...(warning ? { warning } : {})
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
