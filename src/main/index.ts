import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import {
  app,
  BaseWindow,
  BrowserWindow,
  WebContentsView,
  type Session
} from 'electron'
import { applyCleanUserAgent, getCleanUserAgent } from './user-agent'
import {
  createEphemeralSession,
  setupSessionPermissions
} from './session'
import { IPC } from '../shared/ipc-channels'
import { registerIpcHandlers, notifyDevModeChanged } from './ipc'
import { isPhotosGoogleUrl } from '../engine/selectors'
import { getActiveProfileId } from './profile-store'
import { applyZoom, getZoomState, stepZoom, ZOOM_DEFAULT } from './zoom'
import { activateProfileOnSession, loadPhotosWithProfile } from './profile-load'
import {
  initTutorialOverlay,
  layoutTutorialOverlay,
  showTutorialOverlay,
  hideTutorialOverlay,
  renderTutorialOverlay
} from './tutorial-overlay-view'
import {
  setupApplicationMenu,
  setDevModeChangeHandler,
  updateDevModeMenuItem
} from './menu'
import { runStartupUpdateCheck } from './auto-updater'
import { setPrepareAppQuit } from './app-quit'

const PANEL_WIDTH = 320
const PHOTOS_URL = 'https://photos.google.com/'

let baseWindow: BaseWindow | null = null
let photosView: WebContentsView | null = null
let panelView: WebContentsView | null = null
let logWindow: BrowserWindow | null = null
let photosSession: Session | null = null

function getPreload(name: string): string {
  const path = resolve(__dirname, '../preload', `${name}.js`)
  if (!existsSync(path)) {
    console.error(`[preload] Missing: ${path}`)
  }
  return path
}

function getRendererHtml(name: string): string {
  return join(__dirname, '../renderer', name, 'index.html')
}

function layoutViews(): void {
  if (!baseWindow || !photosView || !panelView) return
  const [width, height] = baseWindow.getContentSize()
  const panelW = Math.min(PANEL_WIDTH, width)
  photosView.setBounds({ x: 0, y: 0, width: width - panelW, height })
  panelView.setBounds({ x: width - panelW, y: 0, width: panelW, height })
  layoutTutorialOverlay(width, height)
}

function notifyPanelZoom(state: { factor: number; percent: number }): void {
  panelView?.webContents.send(IPC.PANEL_ZOOM_CHANGED, state)
}

function setupPhotosZoom(webContents: Electron.WebContents): void {
  notifyPanelZoom(applyZoom(webContents, ZOOM_DEFAULT))
  webContents.once('did-finish-load', () => {
    notifyPanelZoom(applyZoom(webContents, ZOOM_DEFAULT))
  })

  webContents.on('before-input-event', (event, input) => {
    if (input.type !== 'mouseWheel') return
    const hasZoomModifier = input.control || input.meta
    if (!hasZoomModifier) return

    event.preventDefault()
    notifyPanelZoom(stepZoom(webContents, input.deltaY > 0 ? -1 : 1))
  })

  webContents.on('zoom-changed', () => {
    notifyPanelZoom(getZoomState(webContents))
  })
}

function wirePhotosViewEvents(view: WebContentsView): void {
  const wc = view.webContents
  setupPhotosZoom(wc)

  wc.on('did-start-navigation', () => {
    void wc
      .executeJavaScript(
        `try { Object.defineProperty(navigator, 'webdriver', { get: () => false }); } catch (_) {}`
      )
      .catch(() => {})
  })

  wc.setWindowOpenHandler(({ url }) => {
    if (isPhotosGoogleUrl(url) || url.startsWith('https://accounts.google.com')) {
      wc.loadURL(url)
    }
    return { action: 'deny' }
  })

  wc.on('did-navigate', pollPageStatus)
  wc.on('did-navigate-in-page', pollPageStatus)
  wc.on('dom-ready', pollPageStatus)
}

function createPhotosView(session: Session): WebContentsView {
  const cleanUa = getCleanUserAgent()
  const view = new WebContentsView({
    webPreferences: {
      preload: getPreload('photos'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
      session
    }
  })
  view.webContents.setUserAgent(cleanUa)
  wirePhotosViewEvents(view)
  return view
}

function createLogWindow(): BrowserWindow {
  if (logWindow && !logWindow.isDestroyed()) return logWindow

  logWindow = new BrowserWindow({
    width: 520,
    height: 400,
    title: 'Log — Google Foto Manager',
    show: false,
    modal: false,
    webPreferences: {
      preload: getPreload('log'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  })

  void (
    process.env.ELECTRON_RENDERER_URL
      ? logWindow.loadURL(`${process.env.ELECTRON_RENDERER_URL}/log/index.html`)
      : logWindow.loadFile(getRendererHtml('log'))
  )

  logWindow.on('closed', () => {
    logWindow = null
  })

  return logWindow
}

function pollPageStatus(): void {
  if (!photosView) return
  photosView.webContents
    .executeJavaScript(`
      (function() {
        const url = window.location.href;
        const text = document.body?.innerText ?? '';
        const blocked = [
          'Questo browser o app potrebbe non essere sicuro',
          'Questo browser o questa app potrebbero non essere sicuri',
          'Impossibile eseguire l\\'accesso',
          'This browser or app may not be secure',
          'browser or app may not be secure'
        ].some(s => text.includes(s));
        return window.__googleFotoEngine?.getPageStatus?.() ?? {
          isPhotosGoogle: /^https:\\/\\/photos\\.google\\.com/i.test(url),
          isLoginBlocked: blocked,
          loginBlockMessage: blocked ? 'Usa «Accedi con Chrome» nel pannello per importare la sessione.' : undefined
        };
      })()
    `)
    .then((status) => {
      panelView?.webContents.send(IPC.PANEL_PAGE_STATUS, status)
    })
    .catch(() => {})
}

export async function switchPhotosSession(profileId: string | null): Promise<void> {
  if (!photosView || !photosSession) return

  await activateProfileOnSession(photosSession, profileId).then((cookieAction) =>
    loadPhotosWithProfile(photosView!.webContents, profileId, cookieAction)
  )
}

function createMainWindow(): void {
  photosSession = createEphemeralSession()
  setupSessionPermissions(photosSession)

  baseWindow = new BaseWindow({
    width: 1280,
    height: 800,
    title: `Google Foto Manager v${app.getVersion()}`,
    show: false
  })

  photosView = createPhotosView(photosSession)

  panelView = new WebContentsView({
    webPreferences: {
      preload: getPreload('panel'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  })

  baseWindow.contentView.addChildView(photosView)
  baseWindow.contentView.addChildView(panelView)
  initTutorialOverlay(baseWindow, panelView)

  layoutViews()
  baseWindow.on('resize', () => {
    layoutViews()
    panelView?.webContents.send(IPC.TUTORIAL_REPOSITION)
  })

  registerIpcHandlers({
    panelView: panelView.webContents,
    photosView: photosView.webContents,
    photosSession,
    getLogWindow: () => logWindow,
    createLogWindow,
    reloadPhotos: () => photosView?.webContents.loadURL(PHOTOS_URL),
    switchPhotosSession
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    panelView.webContents.loadURL(`${process.env.ELECTRON_RENDERER_URL}/panel/index.html`)
  } else {
    panelView.webContents.loadFile(getRendererHtml('panel'))
  }

  const startupProfileId = getActiveProfileId()
  if (startupProfileId) {
    void (async () => {
      if (!photosView || !photosSession) return
      await activateProfileOnSession(photosSession, startupProfileId).then((cookieAction) =>
        loadPhotosWithProfile(photosView!.webContents, startupProfileId, cookieAction)
      )
    })()
  } else {
    photosView.webContents.loadURL(PHOTOS_URL)
  }

  let pageStatusInterval: ReturnType<typeof setInterval> | null = null

  panelView.webContents.once('did-finish-load', () => {
    if (!baseWindow || baseWindow.isDestroyed() || baseWindow.isVisible()) return
    baseWindow.show()
    if (!pageStatusInterval) {
      pageStatusInterval = setInterval(pollPageStatus, 3000)
    }
  })
}

app.commandLine.appendSwitch('disable-blink-features', 'AutomationControlled')
applyCleanUserAgent(app)

setPrepareAppQuit(() => {
  for (const win of BrowserWindow.getAllWindows()) {
    if (!win.isDestroyed()) win.destroy()
  }
  for (const win of BaseWindow.getAllWindows()) {
    if (!win.isDestroyed()) win.destroy()
  }
  baseWindow = null
  photosView = null
  panelView = null
  logWindow = null
})

app.whenReady().then(async () => {
  await runStartupUpdateCheck()

  setDevModeChangeHandler((enabled) => {
    updateDevModeMenuItem(enabled)
    notifyDevModeChanged(enabled)
  })
  setupApplicationMenu()
  createMainWindow()

  app.on('activate', () => {
    if (BaseWindow.getAllWindows().length === 0) createMainWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
