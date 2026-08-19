import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SelectionEngine } from '../../src/engine/selection-engine'
import { createGooglePhotosGrid } from '../fixtures/google-photos-grid'

describe('SelectionEngine', () => {
  let engine: SelectionEngine

  beforeEach(() => {
    vi.useFakeTimers()
    engine = new SelectionEngine()
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { href: 'https://photos.google.com/' }
    })
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('selects all photos and skips Salva tutte', async () => {
    const grid = createGooglePhotosGrid(document, { photoCount: 12, virtualize: false })
    const doneEvents: { total: number; leftovers: number }[] = []
    engine.setCallbacks({ onDone: (e) => doneEvents.push(e) })

    const startPromise = engine.start({
      clickDelay: 0,
      settleDelay: 10,
      scrollFraction: 100,
      passesPerStep: 1,
      stallPasses: 2
    })

    await vi.runAllTimersAsync()
    await startPromise

    expect(grid.getSelectedCount()).toBe(12)
    expect(grid.mainEl.querySelector('[data-clicked="1"]')).toBeNull()
    expect(doneEvents[0]?.total).toBe(12)
  })

  it('counts selections correctly with virtualization', async () => {
    createGooglePhotosGrid(document, { photoCount: 30, viewportHeight: 200, itemHeight: 80 })
    const progress: number[] = []
    engine.setCallbacks({ onProgress: (e) => progress.push(e.selected) })

    const startPromise = engine.start({
      clickDelay: 0,
      settleDelay: 5,
      scrollFraction: 50,
      passesPerStep: 2,
      stallPasses: 3
    })

    await vi.runAllTimersAsync()
    await startPromise

    const maxSelected = Math.max(...progress)
    expect(maxSelected).toBe(30)
  })

  it('honors pause and stop', async () => {
    createGooglePhotosGrid(document, { photoCount: 50, virtualize: false })
    const resets: { state: string; selected: number; estPct: number }[] = []
    engine.setCallbacks({
      onProgress: (e) => {
        if (e.state === 'idle' && e.selected === 0) {
          resets.push({ state: e.state, selected: e.selected, estPct: e.estPct })
        }
      }
    })

    const startPromise = engine.start({
      clickDelay: 10,
      settleDelay: 10,
      scrollFraction: 100,
      passesPerStep: 1,
      stallPasses: 10
    })

    await vi.advanceTimersByTimeAsync(50)
    engine.pause()
    expect(engine.getState()).toBe('paused')

    engine.resume()
    expect(engine.getState()).toBe('running')

    await vi.advanceTimersByTimeAsync(50)
    const stopPromise = engine.stop()
    await vi.runAllTimersAsync()
    await stopPromise
    expect(engine.getState()).toBe('idle')
    expect(resets.length).toBeGreaterThan(0)

    await vi.runAllTimersAsync()
    await startPromise
  })

  it('deselect clicks Cancella selezione', async () => {
    const grid = createGooglePhotosGrid(document, { photoCount: 5, virtualize: false })
    grid.mainEl.querySelectorAll('[role="checkbox"][data-photo]').forEach((cb) => {
      ;(cb as HTMLElement).click()
    })
    expect(grid.getSelectedCount()).toBe(5)

    const clearBtn = document.createElement('button')
    clearBtn.setAttribute('aria-label', 'Cancella selezione')
    clearBtn.addEventListener('click', () => {
      grid.mainEl
        .querySelectorAll('[role="checkbox"][aria-checked="true"]')
        .forEach((cb) => cb.setAttribute('aria-checked', 'false'))
    })
    document.body.appendChild(clearBtn)

    await engine.deselect()
    expect(grid.getSelectedCount()).toBe(0)
    expect(engine.getState()).toBe('idle')
  })

  it('emits error when selectors missing', async () => {
    document.body.innerHTML = '<div>empty</div>'
    const errors: string[] = []
    engine.setCallbacks({ onError: (e) => errors.push(e.message) })

    await engine.start({ clickDelay: 0, settleDelay: 0, scrollFraction: 80, passesPerStep: 1, stallPasses: 1 })
    expect(errors.length).toBe(1)
    expect(engine.getState()).toBe('error')
  })

  it('emits downloading then done when downloadAfterSelection is enabled', async () => {
    createGooglePhotosGrid(document, { photoCount: 4, virtualize: false })
    const states: string[] = []
    let resolveDownload: ((value: { ok: boolean; method?: string }) => void) | null = null

    engine.setCallbacks({
      onProgress: (e) => states.push(e.state),
      onTriggerDownload: () =>
        new Promise((resolve) => {
          resolveDownload = resolve
        })
    })

    const startPromise = engine.start({
      clickDelay: 0,
      settleDelay: 5,
      scrollFraction: 100,
      passesPerStep: 1,
      stallPasses: 2,
      downloadAfterSelection: true
    })

    await vi.runAllTimersAsync()
    expect(states).toContain('downloading')
    expect(engine.getState()).toBe('downloading')
    expect(resolveDownload).not.toBeNull()

    resolveDownload!({ ok: true, method: 'menu' })
    await vi.runAllTimersAsync()
    await startPromise

    expect(states[states.length - 1]).toBe('done')
    expect(engine.getState()).toBe('done')
    const downloadingIdx = states.indexOf('downloading')
    const doneIdx = states.lastIndexOf('done')
    expect(downloadingIdx).toBeGreaterThanOrEqual(0)
    expect(doneIdx).toBeGreaterThan(downloadingIdx)
  })

  it('deletes without scroll container when photos are already selected', async () => {
    const mainEl = document.createElement('div')
    mainEl.setAttribute('role', 'main')
    document.body.appendChild(mainEl)

    const toolbar = document.createElement('div')
    toolbar.setAttribute('role', 'toolbar')
    toolbar.textContent = '2 elementi selezionati'
    document.body.appendChild(toolbar)

    let deleteClicks = 0
    const deleteBtn = document.createElement('button')
    deleteBtn.setAttribute('aria-label', 'Elimina')
    deleteBtn.addEventListener('click', () => {
      deleteClicks++
      const menu = document.createElement('div')
      menu.setAttribute('role', 'menu')
      const item = document.createElement('div')
      item.setAttribute('role', 'menuitem')
      item.setAttribute('aria-label', 'Sposta nel cestino')
      item.addEventListener('click', () => {
        const dialog = document.createElement('div')
        dialog.setAttribute('role', 'dialog')
        const confirm = document.createElement('button')
        confirm.textContent = 'Sposta nel cestino'
        confirm.addEventListener('click', () => dialog.remove())
        dialog.appendChild(confirm)
        document.body.appendChild(dialog)
      })
      menu.appendChild(item)
      document.body.appendChild(menu)
    })
    document.body.appendChild(deleteBtn)

    const doneEvents: { total: number }[] = []
    engine.setCallbacks({ onDone: (e) => doneEvents.push(e) })

    const deletePromise = engine.startAndDelete({
      clickDelay: 0,
      settleDelay: 0,
      scrollFraction: 80,
      passesPerStep: 1,
      stallPasses: 1
    })

    await vi.runAllTimersAsync()
    await deletePromise

    expect(deleteClicks).toBe(1)
    expect(doneEvents[0]?.total).toBe(2)
    expect(engine.getState()).toBe('done')
  })

  it('downloads then deletes when startAndDelete has downloadAfterSelection', async () => {
    createGooglePhotosGrid(document, { photoCount: 3, virtualize: false })

    const order: string[] = []
    let resolveDownload: ((value: { ok: boolean; method?: string }) => void) | null = null

    let deleteClicks = 0
    const deleteBtn = document.createElement('button')
    deleteBtn.setAttribute('aria-label', 'Elimina')
    deleteBtn.addEventListener('click', () => {
      deleteClicks++
      order.push('delete')
      const dialog = document.createElement('div')
      dialog.setAttribute('role', 'dialog')
      const confirm = document.createElement('button')
      confirm.textContent = 'Sposta nel cestino'
      confirm.addEventListener('click', () => dialog.remove())
      dialog.appendChild(confirm)
      document.body.appendChild(dialog)
    })
    document.body.appendChild(deleteBtn)

    engine.setCallbacks({
      onTriggerDownload: () => {
        order.push('download')
        return new Promise((resolve) => {
          resolveDownload = resolve
        })
      }
    })

    const deletePromise = engine.startAndDelete({
      clickDelay: 0,
      settleDelay: 5,
      scrollFraction: 100,
      passesPerStep: 1,
      stallPasses: 2,
      downloadAfterSelection: true
    })

    await vi.runAllTimersAsync()
    expect(order).toEqual(['download'])
    expect(deleteClicks).toBe(0)
    expect(engine.getState()).toBe('downloading')
    expect(resolveDownload).not.toBeNull()

    resolveDownload!({ ok: true, method: 'menu' })
    await vi.runAllTimersAsync()
    await deletePromise

    expect(order).toEqual(['download', 'delete'])
    expect(deleteClicks).toBe(1)
    expect(engine.getState()).toBe('done')
  })

  it('skips delete when downloadAfterSelection is set but download fails', async () => {
    createGooglePhotosGrid(document, { photoCount: 3, virtualize: false })

    let deleteClicks = 0
    const deleteBtn = document.createElement('button')
    deleteBtn.setAttribute('aria-label', 'Elimina')
    deleteBtn.addEventListener('click', () => {
      deleteClicks++
    })
    document.body.appendChild(deleteBtn)

    engine.setCallbacks({
      onTriggerDownload: async () => ({ ok: false, step: 'download-failed' })
    })

    const deletePromise = engine.startAndDelete({
      clickDelay: 0,
      settleDelay: 5,
      scrollFraction: 100,
      passesPerStep: 1,
      stallPasses: 2,
      downloadAfterSelection: true
    })

    await vi.runAllTimersAsync()
    await deletePromise

    expect(deleteClicks).toBe(0)
    expect(engine.getState()).toBe('done')
  })
})
