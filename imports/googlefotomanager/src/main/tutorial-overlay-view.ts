import { WebContentsView, type BaseWindow } from 'electron'
import { join, resolve } from 'node:path'
import { IPC } from '../shared/ipc-channels'
import type {
  TutorialCardLayout,
  TutorialRect,
  TutorialRenderPayload,
  TutorialRenderRequest
} from '../shared/tutorial-types'

const PANEL_WIDTH = 320
const SPOTLIGHT_PAD = 6
const CARD_MARGIN = 14

let overlayView: WebContentsView | null = null
let panelView: WebContentsView | null = null
let baseWindow: BaseWindow | null = null

function getPreload(name: string): string {
  return resolve(__dirname, '../preload', `${name}.js`)
}

function getRendererHtml(name: string): string {
  return join(__dirname, '../renderer', name, 'index.html')
}

export function initTutorialOverlay(
  window: BaseWindow,
  panel: WebContentsView
): WebContentsView {
  baseWindow = window
  panelView = panel

  overlayView = new WebContentsView({
    webPreferences: {
      preload: getPreload('tutorial-overlay'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  })

  overlayView.setBackgroundColor('#00000000')
  overlayView.setVisible(false)
  window.contentView.addChildView(overlayView)

  void (
    process.env.ELECTRON_RENDERER_URL
      ? overlayView.webContents.loadURL(
          `${process.env.ELECTRON_RENDERER_URL}/tutorial-overlay/index.html`
        )
      : overlayView.webContents.loadFile(getRendererHtml('tutorial-overlay'))
  )

  return overlayView
}

export function layoutTutorialOverlay(width: number, height: number): void {
  overlayView?.setBounds({ x: 0, y: 0, width, height })
}

export function showTutorialOverlay(): void {
  overlayView?.setVisible(true)
}

export function hideTutorialOverlay(): void {
  overlayView?.setVisible(false)
}

async function getTargetBounds(selector: string): Promise<TutorialRect | null> {
  if (!panelView) return null

  const local = await panelView.webContents.executeJavaScript(`
    (() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return null;
      el.scrollIntoView({ block: 'nearest', behavior: 'auto' });
      const r = el.getBoundingClientRect();
      return { top: r.top, left: r.left, width: r.width, height: r.height };
    })()
  `)

  if (!local) return null

  const panelBounds = panelView.getBounds()
  return {
    x: panelBounds.x + local.left - SPOTLIGHT_PAD,
    y: panelBounds.y + local.top - SPOTLIGHT_PAD,
    width: local.width + SPOTLIGHT_PAD * 2,
    height: local.height + SPOTLIGHT_PAD * 2
  }
}

function computeCardLayout(
  spotlight: TutorialRect | null,
  windowWidth: number,
  windowHeight: number
): TutorialCardLayout {
  if (!spotlight) {
    return {
      top: Math.round(windowHeight * 0.5),
      left: Math.round(windowWidth * 0.5),
      maxWidth: Math.min(420, windowWidth - CARD_MARGIN * 2),
      centered: true
    }
  }

  const panelLeft = Math.max(0, windowWidth - PANEL_WIDTH)
  const cardMaxWidth = Math.min(360, panelLeft - CARD_MARGIN * 3)
  const cardWidth = Math.max(240, cardMaxWidth)
  const cardHeightEstimate = 180

  let top = spotlight.y
  let left = CARD_MARGIN

  if (panelLeft > CARD_MARGIN * 2 + cardWidth) {
    left = Math.max(CARD_MARGIN, panelLeft - cardWidth - CARD_MARGIN)
  } else {
    left = CARD_MARGIN
  }

  if (top + cardHeightEstimate > windowHeight - CARD_MARGIN) {
    top = Math.max(CARD_MARGIN, windowHeight - cardHeightEstimate - CARD_MARGIN)
  }

  return { top, left, maxWidth: cardWidth }
}

export async function renderTutorialOverlay(
  request: TutorialRenderRequest
): Promise<void> {
  if (!overlayView || !baseWindow) return

  const [windowWidth, windowHeight] = baseWindow.getContentSize()
  const spotlight =
    request.mode === 'step' && request.targetSelector
      ? await getTargetBounds(request.targetSelector)
      : null

  const payload: TutorialRenderPayload = {
    mode: request.mode,
    html: request.html,
    stepLabel: request.stepLabel,
    skipLabel: request.skipLabel,
    nextLabel: request.nextLabel,
    spotlight,
    card: computeCardLayout(spotlight, windowWidth, windowHeight)
  }

  if (overlayView.webContents.isLoading()) {
    overlayView.webContents.once('did-finish-load', () => {
      overlayView?.webContents.send(IPC.TUTORIAL_RENDER, payload)
    })
  } else {
    overlayView.webContents.send(IPC.TUTORIAL_RENDER, payload)
  }
}

export function getTutorialOverlayWebContents(): Electron.WebContents | null {
  return overlayView?.webContents ?? null
}
