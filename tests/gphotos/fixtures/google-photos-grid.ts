export interface GridOptions {
  photoCount?: number
  skipLabel?: string
  viewportHeight?: number
  itemHeight?: number
  virtualize?: boolean
  /** ready: every preview is already loaded. manual: tiles mounted after the first paint stay unloaded until releasePreviews(). */
  previews?: 'ready' | 'manual'
}

export function createGooglePhotosGrid(doc: Document, options: GridOptions = {}): {
  scrollEl: HTMLElement
  mainEl: HTMLElement
  getSelectedCount: () => number
  simulateScroll: (delta: number) => void
  releasePreviews: () => void
} {
  const {
    photoCount = 20,
    skipLabel = 'Salva tutte le foto del giorno',
    viewportHeight = 400,
    itemHeight = 80,
    virtualize = true,
    previews
  } = options
  let previewsReleased = previews === 'ready'
  const firstMounted = new Set<number>()
  let firstRender = true

  const setPreview = (img: HTMLImageElement, ready: boolean): void => {
    Object.defineProperty(img, 'complete', { configurable: true, get: () => ready })
    Object.defineProperty(img, 'naturalWidth', { configurable: true, get: () => (ready ? 8 : 0) })
  }

  const ensurePreview = (cb: HTMLElement, index: number): void => {
    if (!previews) return
    let img = cb.querySelector('img')
    if (!img) {
      img = doc.createElement('img')
      img.alt = ''
      img.setAttribute('src', `photo-${index}.jpg`)
      cb.appendChild(img)
    }
    setPreview(img, previewsReleased || firstMounted.has(index))
  }

  doc.body.innerHTML = ''

  const scrollEl = doc.createElement('c-wiz')
  scrollEl.style.overflowY = 'auto'
  scrollEl.style.height = `${viewportHeight}px`
  scrollEl.style.display = 'block'

  const mainEl = doc.createElement('div')
  mainEl.setAttribute('role', 'main')

  const skipBtn = doc.createElement('div')
  skipBtn.setAttribute('role', 'checkbox')
  skipBtn.setAttribute('aria-checked', 'false')
  skipBtn.setAttribute('aria-label', skipLabel)
  skipBtn.addEventListener('click', () => {
    const checked = skipBtn.getAttribute('aria-checked') === 'true'
    skipBtn.setAttribute('aria-checked', checked ? 'false' : 'true')
    if (!checked) skipBtn.dataset.clicked = '1'
  })
  mainEl.appendChild(skipBtn)

  const photos: HTMLElement[] = []
  for (let i = 0; i < photoCount; i++) {
    const cb = doc.createElement('div')
    cb.setAttribute('role', 'checkbox')
    cb.setAttribute('aria-checked', 'false')
    cb.setAttribute('aria-label', `Foto ${i + 1}`)
    cb.style.height = `${itemHeight}px`
    cb.addEventListener('click', () => {
      const checked = cb.getAttribute('aria-checked') === 'true'
      cb.setAttribute('aria-checked', checked ? 'false' : 'true')
    })
    photos.push(cb)
  }

  const totalHeight = photoCount * itemHeight + 200
  scrollEl.style.height = `${viewportHeight}px`

  const renderVisible = (): void => {
    mainEl.querySelectorAll('[data-photo]').forEach((n) => n.remove())
    if (!virtualize) {
      photos.forEach((p, idx) => {
        p.dataset.photo = String(idx)
        if (firstRender) firstMounted.add(idx)
        ensurePreview(p, idx)
        mainEl.appendChild(p)
      })
      scrollEl.style.height = `${viewportHeight}px`
      Object.defineProperty(scrollEl, 'scrollHeight', { configurable: true, value: totalHeight })
      Object.defineProperty(scrollEl, 'clientHeight', { configurable: true, value: viewportHeight })
      return
    }

    const scrollTop = scrollEl.scrollTop
    const visibleCount = Math.ceil(viewportHeight / itemHeight) + 2
    const startIdx = Math.floor(scrollTop / itemHeight)
    for (let i = startIdx; i < Math.min(startIdx + visibleCount, photos.length); i++) {
      const p = photos[i]
      p.dataset.photo = String(i)
      if (firstRender) firstMounted.add(i)
      ensurePreview(p, i)
      mainEl.appendChild(p)
    }

    Object.defineProperty(scrollEl, 'scrollHeight', { configurable: true, value: totalHeight })
    Object.defineProperty(scrollEl, 'clientHeight', { configurable: true, value: viewportHeight })
  }

  scrollEl.appendChild(mainEl)
  doc.body.appendChild(scrollEl)

  scrollEl.addEventListener('scroll', renderVisible)
  scrollEl.scrollBy = (options?: ScrollToOptions | number, _y?: number) => {
    const delta =
      typeof options === 'number'
        ? options
        : typeof options === 'object' && options?.top != null
          ? options.top
          : 0
    scrollEl.scrollTop = Math.min(
      Math.max(0, scrollEl.scrollTop + delta),
      scrollEl.scrollHeight - scrollEl.clientHeight
    )
    scrollEl.dispatchEvent(new Event('scroll'))
  }
  renderVisible()
  firstRender = false

  return {
    scrollEl,
    mainEl,
    getSelectedCount: () =>
      photos.filter((p) => p.getAttribute('aria-checked') === 'true').length,
    simulateScroll: (delta: number) => {
      scrollEl.scrollTop = Math.min(
        scrollEl.scrollTop + delta,
        scrollEl.scrollHeight - scrollEl.clientHeight
      )
      scrollEl.dispatchEvent(new Event('scroll'))
    },
    releasePreviews: () => {
      previewsReleased = true
      for (const p of photos) {
        const img = p.querySelector('img')
        if (img) {
          setPreview(img, true)
          img.dispatchEvent(new Event('load', { bubbles: true }))
        }
      }
    }
  }
}
