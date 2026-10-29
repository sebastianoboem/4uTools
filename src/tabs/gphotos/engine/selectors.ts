export const SELECTORS = {
  scrollContainerTag: 'c-wiz',
  mainRole: '[role="main"]',
  checkboxRole: 'div[role="checkbox"]',
  clearSelectionLabel: 'Cancella selezione'
} as const

export const CLEAR_SELECTION_LABELS = [
  SELECTORS.clearSelectionLabel,
  'Clear selection'
] as const

export const MORE_OPTIONS_LABELS = [
  'Altre opzioni',
  'More options',
  'Altre azioni',
  'More actions'
] as const

export const DOWNLOAD_LABELS = ['Scarica', 'Download'] as const

export const DELETE_TOOLBAR_LABELS = [
  'Elimina',
  'Delete',
  'Sposta nel cestino',
  'Move to trash',
  'Sposta nel cestino del dispositivo'
] as const

export const DELETE_SUBMENU_LABELS = [
  'Sposta nel cestino',
  'Move to trash',
  'Elimina',
  'Delete',
  'Sposta nel cestino del dispositivo'
] as const

export const DELETE_CONFIRM_LABELS = [
  'Sposta nel cestino',
  'Move to trash',
  'Elimina',
  'Delete',
  'Conferma',
  'OK',
  'Ok'
] as const

export const DELETE_CANCEL_LABELS = ['Annulla', 'Cancel', 'No'] as const

export const LOGIN_BLOCK_TEXTS = [
  'Questo browser o app potrebbe non essere sicuro',
  'Questo browser o questa app potrebbero non essere sicuri',
  'Impossibile eseguire l\'accesso',
  'This browser or app may not be secure',
  'browser or app may not be secure',
  'Couldn\'t sign you in'
] as const

export function findScrollContainer(doc: Document = document): HTMLElement | null {
  const view = doc.defaultView
  const main = findMainElement(doc)

  if (main) {
    let el: HTMLElement | null = main
    while (el) {
      const style = view?.getComputedStyle(el)
      if (style && isVerticallyScrollable(el, style)) return el
      el = el.parentElement
    }
  }

  const candidates = [...doc.querySelectorAll<HTMLElement>(SELECTORS.scrollContainerTag)]
  let best: HTMLElement | null = null
  let bestOverflow = 0

  for (const el of candidates) {
    const style = view?.getComputedStyle(el)
    if (!style) continue
    const overflow = el.scrollHeight - el.clientHeight
    if (overflow <= 0) continue
    if (hasScrollableOverflow(style) && overflow > bestOverflow) {
      bestOverflow = overflow
      best = el
    }
  }
  if (best) return best

  if (main) {
    for (const el of candidates) {
      if (el.contains(main) && el.scrollHeight > el.clientHeight + 1) return el
    }
  }

  const scrollingEl = doc.scrollingElement as HTMLElement | null
  if (scrollingEl && scrollingEl.scrollHeight > scrollingEl.clientHeight + 1) {
    return scrollingEl
  }

  return null
}

function hasScrollableOverflow(style: CSSStyleDeclaration): boolean {
  const oy = style.overflowY
  return oy === 'auto' || oy === 'scroll' || oy === 'overlay' || oy === 'hidden'
}

function isVerticallyScrollable(el: HTMLElement, style: CSSStyleDeclaration): boolean {
  return hasScrollableOverflow(style) && el.scrollHeight > el.clientHeight + 1
}

export function findMainElement(doc: Document = document): HTMLElement | null {
  const all = [...doc.querySelectorAll<HTMLElement>(SELECTORS.mainRole)]
  if (all.length === 0) return null
  if (all.length === 1) return all[0] ?? null

  let best: HTMLElement | null = null
  let bestCount = -1
  for (const el of all) {
    const count = el.querySelectorAll(SELECTORS.checkboxRole).length
    if (count > bestCount) {
      bestCount = count
      best = el
    }
  }
  return best
}

/** Google Foto toolbar button to clear the current multi-selection. */
export function findClearSelectionButton(doc: Document = document): HTMLElement | null {
  for (const label of CLEAR_SELECTION_LABELS) {
    const btn = doc.querySelector<HTMLElement>(`button[aria-label="${label}"]`)
    if (btn) return btn
  }
  return doc.querySelector<HTMLElement>(`[aria-label="${SELECTORS.clearSelectionLabel}"]`)
}

function isVisibleInteractive(el: HTMLElement): boolean {
  const rect = el.getBoundingClientRect()
  if (rect.width <= 0 || rect.height <= 0) return false
  const style = el.ownerDocument.defaultView?.getComputedStyle(el)
  if (!style || style.visibility === 'hidden' || style.display === 'none') return false
  if (el.getAttribute('aria-disabled') === 'true' || el.hasAttribute('disabled')) return false
  return true
}

function findButtonByLabels(labels: readonly string[], doc: Document = document): HTMLElement | null {
  const matches: HTMLElement[] = []
  for (const label of labels) {
    const btn = doc.querySelector<HTMLElement>(`button[aria-label="${label}"]`)
    if (btn) matches.push(btn)
  }
  return matches.find(isVisibleInteractive) ?? matches[0] ?? null
}

export function findMoreOptionsButton(doc: Document = document): HTMLElement | null {
  return findButtonByLabels(MORE_OPTIONS_LABELS, doc)
}

export function findDownloadMenuItem(doc: Document = document): HTMLElement | null {
  for (const label of DOWNLOAD_LABELS) {
    const byRole = doc.querySelector<HTMLElement>(`[role="menuitem"][aria-label="${label}"]`)
    if (byRole) return byRole
    const byText = [...doc.querySelectorAll<HTMLElement>('[role="menuitem"], [role="option"]')].find(
      (el) => el.textContent?.trim() === label
    )
    if (byText) return byText
  }
  return null
}

export function findDeleteButton(doc: Document = document): HTMLElement | null {
  return findButtonByLabels(DELETE_TOOLBAR_LABELS, doc)
}

export function findDeleteMenuItem(doc: Document = document): HTMLElement | null {
  const matches: HTMLElement[] = []
  for (const label of DELETE_SUBMENU_LABELS) {
    const byRole = doc.querySelector<HTMLElement>(`[role="menuitem"][aria-label="${label}"]`)
    if (byRole) matches.push(byRole)
    const byOption = doc.querySelector<HTMLElement>(`[role="option"][aria-label="${label}"]`)
    if (byOption) matches.push(byOption)
    const byText = [...doc.querySelectorAll<HTMLElement>('[role="menuitem"], [role="option"]')].find(
      (el) => {
        const text = el.textContent?.trim() ?? ''
        return text === label || text.startsWith(`${label} `)
      }
    )
    if (byText) matches.push(byText)
  }
  return matches.find(isVisibleInteractive) ?? matches[0] ?? null
}

export function findDeleteConfirmButton(doc: Document = document): HTMLElement | null {
  const dialog = doc.querySelector<HTMLElement>('[role="dialog"], [role="alertdialog"]')
  if (!dialog) return null

  const visibleButtons = [...dialog.querySelectorAll<HTMLElement>('button')].filter(isVisibleInteractive)

  for (const label of DELETE_CONFIRM_LABELS) {
    const btn = visibleButtons.find(
      (el) => el.getAttribute('aria-label') === label || el.textContent?.trim() === label
    )
    if (btn) return btn
  }

  const affirmative = visibleButtons.find((btn) => {
    const text = btn.textContent?.trim() ?? ''
    const aria = btn.getAttribute('aria-label') ?? ''
    return !DELETE_CANCEL_LABELS.some((label) => text === label || aria === label)
  })

  return affirmative ?? null
}

export function countCheckboxes(doc: Document = document): number {
  const main = findMainElement(doc)
  if (!main) return 0
  return main.querySelectorAll(SELECTORS.checkboxRole).length
}

export function countScrollContainers(doc: Document = document): number {
  return doc.querySelectorAll<HTMLElement>(SELECTORS.scrollContainerTag).length
}

export function isSkipLabel(label: string, skipPrefix: string): boolean {
  return label.startsWith(skipPrefix)
}

export function detectLoginBlock(doc: Document = document): { blocked: boolean; message?: string } {
  const text = doc.body?.innerText ?? ''
  for (const snippet of LOGIN_BLOCK_TEXTS) {
    if (text.includes(snippet)) {
      return { blocked: true, message: snippet }
    }
  }
  return { blocked: false }
}

export function isPhotosGoogleUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    return parsed.hostname === 'photos.google.com'
  } catch {
    return false
  }
}

export function gatherDiagnostics(doc: Document = document, url?: string) {
  return {
    scrollContainers: countScrollContainers(doc),
    mainElements: doc.querySelectorAll(SELECTORS.mainRole).length,
    checkboxes: countCheckboxes(doc),
    url
  }
}

const SELECTION_COUNT_PATTERNS: RegExp[] = [
  /(\d[\d.,\s\u00a0\u202f]*)\s+element(?:o|i)\s+selezionat/i,
  /(\d[\d.,\s\u00a0\u202f]*)\s+items?\s+selected/i,
  /(\d[\d.,\s\u00a0\u202f]*)\s+(?:foto|photo|file)\s+selezionat/i,
  /(\d[\d.,\s\u00a0\u202f]*)\s+selected/i
]

/** Parses locale-formatted integers (e.g. 1.389 → 1389). */
export function parseLocalizedInteger(raw: string): number {
  const normalized = raw.trim().replace(/[\s\u00a0\u202f]/g, '')
  if (/^\d{1,3}(\.\d{3})+$/.test(normalized)) {
    return parseInt(normalized.replace(/\./g, ''), 10)
  }
  if (/^\d{1,3}(,\d{3})+$/.test(normalized)) {
    return parseInt(normalized.replace(/,/g, ''), 10)
  }
  const digits = normalized.replace(/[^\d]/g, '')
  return digits ? parseInt(digits, 10) : 0
}

export function parseSelectionCountText(text: string): number | null {
  for (const pattern of SELECTION_COUNT_PATTERNS) {
    const match = text.match(pattern)
    if (match?.[1]) {
      const value = parseLocalizedInteger(match[1])
      if (value >= 0) return value
    }
  }
  return null
}

/** Reads the selection total shown by Google Foto (toolbar/header), not DOM checkbox clicks. */
export function readGooglePhotosSelectionCount(doc: Document = document): number | null {
  const scanText = (text: string): number | null => {
    const value = parseSelectionCountText(text)
    return value !== null && value > 0 ? value : null
  }

  for (const toolbar of doc.querySelectorAll('[role="toolbar"]')) {
    const fromToolbar = scanText(toolbar.textContent ?? '')
    if (fromToolbar !== null) return fromToolbar
    for (const el of toolbar.querySelectorAll('[aria-label]')) {
      const fromLabel = scanText(el.getAttribute('aria-label') ?? '')
      if (fromLabel !== null) return fromLabel
    }
  }

  for (const header of doc.querySelectorAll('header')) {
    const fromHeader = scanText(header.textContent ?? '')
    if (fromHeader !== null) return fromHeader
  }

  return scanText(doc.body?.innerText?.slice(0, 12_000) ?? '')
}
