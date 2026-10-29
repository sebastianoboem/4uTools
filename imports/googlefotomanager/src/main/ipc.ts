import { app, BrowserWindow, ipcMain, session as electronSession, type Session, WebContents } from 'electron'
import { IPC } from '../shared/ipc-channels'
import type {
  LogEvent,
  PageStatus,
  ProgressEvent,
  SelectionParams
} from '../shared/types'
import { runChromeLoginBridge } from './chrome-login-bridge'
import { triggerPhotosDownload } from './download-trigger'
import { waitForSessionDownload } from './download-monitor'
import {
  deleteProfileStorage,
  exportWebStorage,
  saveProfileStorage
} from './profile-storage'
import { deleteProfileCookies, saveProfileCookies } from './profile-cookies'
import {
  createProfile,
  deleteProfile,
  listProfiles,
  setActiveProfile
} from './profile-store'
import { loadSettings, resetSettings, saveSettings } from './settings-store'
import { getDevMode, getTutorialCompleted, setTutorialCompleted } from './app-settings'
import { partitionForProfile } from './session'
import { getZoomState, resetZoom, stepZoom } from './zoom'
import type { TutorialRenderRequest } from '../shared/tutorial-types'
import {
  hideTutorialOverlay,
  renderTutorialOverlay,
  showTutorialOverlay
} from './tutorial-overlay-view'

type WindowRefs = {
  panelView: WebContents | null
  photosView: WebContents | null
  photosSession: Session | null
  getLogWindow: () => BrowserWindow | null
  createLogWindow: () => BrowserWindow
  reloadPhotos: () => void
  switchPhotosSession: (profileId: string | null) => Promise<void>
}

let refs: WindowRefs | null = null
const LOG_BUFFER_MAX = 500
const logBuffer: LogEvent[] = []
let lastProgress: ProgressEvent = {
  selected: 0,
  estPct: 0,
  elapsedMs: 0,
  state: 'idle'
}

export function updatePhotosViewRef(photosView: WebContents, photosSession: Session): void {
  if (!refs) return
  refs.photosView = photosView
  refs.photosSession = photosSession
}

function appendLog(event: LogEvent): void {
  logBuffer.push(event)
  if (logBuffer.length > LOG_BUFFER_MAX) logBuffer.shift()

  const logWin = refs?.getLogWindow()
  if (logWin && !logWin.isDestroyed() && !logWin.webContents.isLoading()) {
    logWin.webContents.send(IPC.LOG_APPEND, event)
  }
}

function ensureLogWindow(): BrowserWindow | null {
  try {
    const existing = refs?.getLogWindow()
    if (existing && !existing.isDestroyed()) {
      const showWindow = (): void => {
        existing.show()
        existing.focus()
      }
      if (existing.webContents.isLoading()) {
        existing.webContents.once('did-finish-load', showWindow)
      } else {
        showWindow()
      }
      return existing
    }
    if (!refs?.createLogWindow) return null
    const win = refs.createLogWindow()
    win.webContents.once('did-finish-load', () => {
      win.show()
      win.focus()
    })
    return win
  } catch (err) {
    console.error('[log] ensureLogWindow failed:', err)
    return null
  }
}

export function notifyDevModeChanged(enabled: boolean): void {
  refs?.panelView?.send(IPC.PANEL_DEV_MODE_CHANGED, { enabled })
}

export function registerIpcHandlers(windowRefs: WindowRefs): void {
  refs = windowRefs

  ipcMain.handle(IPC.PANEL_GET_SETTINGS, () => loadSettings())

  ipcMain.handle(IPC.PANEL_SAVE_SETTINGS, (_e, params: SelectionParams) => saveSettings(params))

  ipcMain.handle(IPC.PANEL_RESET_SETTINGS, () => resetSettings())

  ipcMain.handle(IPC.PANEL_START, async (_e, params: SelectionParams) => {
    saveSettings(params)
    if (!refs?.photosView) throw new Error('Browser non pronto.')
    refs.photosView.send(IPC.PHOTOS_START, params)
  })

  ipcMain.handle(IPC.PANEL_DELETE, async (_e, params: SelectionParams) => {
    saveSettings(params)
    if (!refs?.photosView) throw new Error('Browser non pronto.')
    refs.photosView.send(IPC.PHOTOS_DELETE, params)
  })

  ipcMain.on(IPC.PANEL_PAUSE, () => refs?.photosView?.send(IPC.PHOTOS_PAUSE))
  ipcMain.on(IPC.PANEL_RESUME, () => refs?.photosView?.send(IPC.PHOTOS_RESUME))
  ipcMain.on(IPC.PANEL_STOP, () => refs?.photosView?.send(IPC.PHOTOS_STOP))
  ipcMain.on(IPC.PANEL_DESELECT, () => refs?.photosView?.send(IPC.PHOTOS_DESELECT))

  ipcMain.handle(IPC.PANEL_REDIAGNOSE, async () => {
    if (!refs?.photosView) return { scrollContainers: 0, mainElements: 0, checkboxes: 0 }
    return refs.photosView.executeJavaScript(
      `window.__googleFotoEngine?.diagnose?.() ?? { scrollContainers: 0, mainElements: 0, checkboxes: 0 }`
    )
  })

  ipcMain.handle(IPC.PANEL_GET_PAGE_STATUS, async (): Promise<PageStatus> => {
    if (!refs?.photosView) {
      return { isPhotosGoogle: false, isLoginBlocked: false }
    }
    return refs.photosView.executeJavaScript(`
      window.__googleFotoEngine?.getPageStatus?.() ?? { isPhotosGoogle: false, isLoginBlocked: false }
    `)
  })

  ipcMain.handle(IPC.PANEL_CHROME_LOGIN, async () => {
    if (!refs?.photosSession) {
      return { ok: false, message: 'Sessione non disponibile.' }
    }
    appendLog({ ts: Date.now(), level: 'azione', msg: 'Avvio login tramite Chrome…' })

    const result = await runChromeLoginBridge(refs.photosSession, (msg) => {
      appendLog({ ts: Date.now(), level: 'info', msg })
    })

    if (result.ok) {
      appendLog({
        ts: Date.now(),
        level: 'info',
        msg: `Sessione importata (${result.cookieCount} cookie). Ricarico Google Foto…`
      })
      refs.reloadPhotos()
    } else {
      appendLog({ ts: Date.now(), level: 'errore', msg: result.message })
    }

    return result
  })

  ipcMain.handle(IPC.PANEL_LIST_PROFILES, () => listProfiles())

  ipcMain.handle(IPC.PANEL_SWITCH_PROFILE, async (_e, profileId: string | null) => {
    const store = setActiveProfile(profileId)
    await refs?.switchPhotosSession(profileId)
    appendLog({
      ts: Date.now(),
      level: 'info',
      msg:
        profileId === null
          ? 'Passato a sessione temporanea (non salvata).'
          : `Profilo «${store.profiles.find((p) => p.id === profileId)?.name ?? profileId}» caricato.`
    })
    return store
  })

  ipcMain.handle(IPC.PANEL_SAVE_PROFILE, async (_e, name: string) => {
    if (!refs?.photosSession || !refs.photosView) {
      return { ok: false, message: 'Sessione non disponibile.' }
    }

    const trimmed = name.trim()
    if (!trimmed) {
      return { ok: false, message: 'Inserisci un nome per la sessione.' }
    }

    const profile = createProfile(trimmed)
    const cookies = await refs.photosSession.cookies.get({})
    if (cookies.length === 0) {
      deleteProfile(profile.id)
      return {
        ok: false,
        message: 'Nessun cookie da salvare. Accedi prima con Chrome.'
      }
    }

    try {
      const storage = await exportWebStorage(refs.photosView)
      saveProfileStorage(profile.id, storage)
      saveProfileCookies(profile.id, cookies)
    } catch {
      // storage export is best-effort
    }

    const store = setActiveProfile(profile.id)
    appendLog({
      ts: Date.now(),
      level: 'info',
      msg: `Profilo «${profile.name}» salvato (${cookies.length} cookie).`
    })

    return { ok: true, profile, store, cookieCount: cookies.length }
  })

  ipcMain.handle(IPC.PANEL_DELETE_PROFILE, async (_e, profileId: string) => {
    const store = listProfiles()
    const profile = store.profiles.find((p) => p.id === profileId)
    if (!profile) return { ok: false, message: 'Profilo non trovato.' }

    const wasActive = store.activeProfileId === profileId
    deleteProfile(profileId)

    try {
      const ses = electronSession.fromPartition(partitionForProfile(profileId))
      await ses.clearStorageData()
      await ses.clearCache()
    } catch {
      // ignore cleanup errors
    }
    deleteProfileStorage(profileId)
    deleteProfileCookies(profileId)

    if (wasActive) {
      setActiveProfile(null)
      await refs?.switchPhotosSession(null)
    }

    appendLog({
      ts: Date.now(),
      level: 'info',
      msg: `Profilo «${profile.name}» eliminato.`
    })

    return { ok: true, store: listProfiles() }
  })

  ipcMain.handle(IPC.PANEL_GET_DEV_MODE, () => ({ enabled: getDevMode() }))

  ipcMain.handle(IPC.PANEL_GET_APP_VERSION, () => app.getVersion())

  ipcMain.handle(IPC.PANEL_GET_TUTORIAL_COMPLETED, () => ({
    completed: getTutorialCompleted()
  }))

  ipcMain.handle(IPC.PANEL_SET_TUTORIAL_COMPLETED, (_e, completed: boolean) => {
    setTutorialCompleted(completed)
    return { completed }
  })

  ipcMain.on(IPC.TUTORIAL_SHOW, () => {
    showTutorialOverlay()
  })

  ipcMain.on(IPC.TUTORIAL_HIDE, () => {
    hideTutorialOverlay()
  })

  ipcMain.handle(IPC.TUTORIAL_RENDER_REQUEST, (_e, request: TutorialRenderRequest) => {
    void renderTutorialOverlay(request)
  })

  ipcMain.on(IPC.TUTORIAL_ACTION, (_e, payload) => {
    refs?.panelView?.send(IPC.TUTORIAL_ACTION, payload)
  })

  ipcMain.handle(IPC.PANEL_GET_ZOOM, () => getZoomState(refs?.photosView ?? null))

  ipcMain.handle(IPC.PANEL_ZOOM_STEP, (_e, direction: -1 | 1) => {
    if (!refs?.photosView) return getZoomState(null)
    return stepZoom(refs.photosView, direction)
  })

  ipcMain.handle(IPC.PANEL_ZOOM_RESET, () => {
    if (!refs?.photosView) return getZoomState(null)
    return resetZoom(refs.photosView)
  })

  ipcMain.on(IPC.PANEL_OPEN_LOG, () => {
    ensureLogWindow()
  })

  ipcMain.on(IPC.PANEL_CLOSE_LOG, () => {
    refs?.getLogWindow()?.hide()
  })

  ipcMain.on(IPC.LOG_REQUEST_CLEAR, () => {
    logBuffer.length = 0
    refs?.getLogWindow()?.webContents.send(IPC.LOG_CLEAR)
  })

  ipcMain.handle(IPC.LOG_GET_HISTORY, () => [...logBuffer])

  ipcMain.on(IPC.PHOTOS_PROGRESS, (_e, payload: ProgressEvent) => {
    lastProgress = payload
    refs?.panelView?.send(IPC.PANEL_PROGRESS, payload)
  })

  ipcMain.on(IPC.PHOTOS_LOG, (_e, payload: LogEvent) => {
    appendLog(payload)
  })

  ipcMain.on(IPC.PHOTOS_DONE, (_e, payload) => {
    refs?.panelView?.send(IPC.PANEL_DONE, payload)
  })

  ipcMain.on(IPC.PHOTOS_ERROR, (_e, payload) => {
    refs?.panelView?.send(IPC.PANEL_ERROR, payload)
  })

  ipcMain.on(IPC.PHOTOS_PAGE_STATUS, (_e, payload: PageStatus) => {
    refs?.panelView?.send(IPC.PANEL_PAGE_STATUS, payload)
  })

  ipcMain.handle(IPC.PHOTOS_TRIGGER_DOWNLOAD, async () => {
    if (!refs?.photosView || !refs.photosSession) {
      return { ok: false, step: 'no-webcontents' }
    }

    const selected = lastProgress.selected
    const elapsedMs = lastProgress.elapsedMs

    appendLog({
      ts: Date.now(),
      level: 'info',
      msg: 'Attendo l\'avvio del download (max 15 minuti)'
    })

    const downloadWait = waitForSessionDownload(refs.photosSession, {
      onProgress: (info) => {
        refs?.panelView?.send(IPC.PANEL_PROGRESS, {
          selected,
          estPct: info.percent,
          elapsedMs,
          state: 'downloading'
        } satisfies ProgressEvent)
      }
    })

    const triggerResult = await triggerPhotosDownload(refs.photosView)
    if (!triggerResult.ok) {
      downloadWait.cancel({ ok: false, step: triggerResult.step ?? 'trigger-failed' })
      await downloadWait.promise
      return triggerResult
    }

    const waitResult = await downloadWait.promise
    if (!waitResult.ok) {
      return {
        ok: false,
        method: triggerResult.method,
        step: waitResult.step ?? 'download-failed'
      }
    }

    refs?.panelView?.send(IPC.PANEL_PROGRESS, {
      selected,
      estPct: 100,
      elapsedMs,
      state: 'downloading'
    } satisfies ProgressEvent)

    return { ok: true, method: triggerResult.method }
  })
}
