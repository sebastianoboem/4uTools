import { contextBridge, ipcRenderer } from 'electron'
import { IPC } from '../shared/ipc-channels'
import type {
  DoneEvent,
  ErrorEvent,
  PageStatus,
  ProfileStoreData,
  ProgressEvent,
  SavedProfile,
  SelectionParams
} from '../shared/types'

contextBridge.exposeInMainWorld('panelApi', {
  getSettings: (): Promise<SelectionParams> => ipcRenderer.invoke(IPC.PANEL_GET_SETTINGS),
  saveSettings: (params: SelectionParams): Promise<SelectionParams> =>
    ipcRenderer.invoke(IPC.PANEL_SAVE_SETTINGS, params),
  resetSettings: (): Promise<SelectionParams> => ipcRenderer.invoke(IPC.PANEL_RESET_SETTINGS),
  start: (params: SelectionParams): Promise<void> => ipcRenderer.invoke(IPC.PANEL_START, params),
  deleteSelected: (params: SelectionParams): Promise<void> =>
    ipcRenderer.invoke(IPC.PANEL_DELETE, params),
  pause: (): void => ipcRenderer.send(IPC.PANEL_PAUSE),
  resume: (): void => ipcRenderer.send(IPC.PANEL_RESUME),
  stop: (): void => ipcRenderer.send(IPC.PANEL_STOP),
  openLog: (): void => ipcRenderer.send(IPC.PANEL_OPEN_LOG),
  rediagnose: () => ipcRenderer.invoke(IPC.PANEL_REDIAGNOSE),
  getPageStatus: (): Promise<PageStatus> => ipcRenderer.invoke(IPC.PANEL_GET_PAGE_STATUS),
  chromeLogin: (): Promise<{ ok: boolean; message?: string; cookieCount?: number }> =>
    ipcRenderer.invoke(IPC.PANEL_CHROME_LOGIN),
  listProfiles: (): Promise<ProfileStoreData> => ipcRenderer.invoke(IPC.PANEL_LIST_PROFILES),
  switchProfile: (profileId: string | null): Promise<ProfileStoreData> =>
    ipcRenderer.invoke(IPC.PANEL_SWITCH_PROFILE, profileId),
  saveProfile: (
    name: string
  ): Promise<{
    ok: boolean
    message?: string
    profile?: SavedProfile
    store?: ProfileStoreData
    cookieCount?: number
  }> => ipcRenderer.invoke(IPC.PANEL_SAVE_PROFILE, name),
  deleteProfile: (
    profileId: string
  ): Promise<{ ok: boolean; message?: string; store?: ProfileStoreData }> =>
    ipcRenderer.invoke(IPC.PANEL_DELETE_PROFILE, profileId),
  getZoom: (): Promise<{ factor: number; percent: number }> =>
    ipcRenderer.invoke(IPC.PANEL_GET_ZOOM),
  getDevMode: (): Promise<{ enabled: boolean }> =>
    ipcRenderer.invoke(IPC.PANEL_GET_DEV_MODE),
  getAppVersion: (): Promise<string> => ipcRenderer.invoke(IPC.PANEL_GET_APP_VERSION),
  getTutorialCompleted: (): Promise<{ completed: boolean }> =>
    ipcRenderer.invoke(IPC.PANEL_GET_TUTORIAL_COMPLETED),
  setTutorialCompleted: (completed: boolean): Promise<{ completed: boolean }> =>
    ipcRenderer.invoke(IPC.PANEL_SET_TUTORIAL_COMPLETED, completed),
  tutorialShow: (): void => ipcRenderer.send(IPC.TUTORIAL_SHOW),
  tutorialHide: (): void => ipcRenderer.send(IPC.TUTORIAL_HIDE),
  tutorialRender: (request: import('../shared/tutorial-types').TutorialRenderRequest): Promise<void> =>
    ipcRenderer.invoke(IPC.TUTORIAL_RENDER_REQUEST, request),
  onTutorialAction: (
    cb: (action: import('../shared/tutorial-types').TutorialActionPayload) => void
  ): (() => void) => {
    const handler = (_: unknown, payload: import('../shared/tutorial-types').TutorialActionPayload) =>
      cb(payload)
    ipcRenderer.on(IPC.TUTORIAL_ACTION, handler)
    return () => ipcRenderer.removeListener(IPC.TUTORIAL_ACTION, handler)
  },
  onTutorialReposition: (cb: () => void): (() => void) => {
    const handler = () => cb()
    ipcRenderer.on(IPC.TUTORIAL_REPOSITION, handler)
    return () => ipcRenderer.removeListener(IPC.TUTORIAL_REPOSITION, handler)
  },
  zoomStep: (direction: -1 | 1): Promise<{ factor: number; percent: number }> =>
    ipcRenderer.invoke(IPC.PANEL_ZOOM_STEP, direction),
  zoomReset: (): Promise<{ factor: number; percent: number }> =>
    ipcRenderer.invoke(IPC.PANEL_ZOOM_RESET),
  onProgress: (cb: (event: ProgressEvent) => void): (() => void) => {
    const handler = (_: unknown, payload: ProgressEvent) => cb(payload)
    ipcRenderer.on(IPC.PANEL_PROGRESS, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_PROGRESS, handler)
  },
  onDone: (cb: (event: DoneEvent) => void): (() => void) => {
    const handler = (_: unknown, payload: DoneEvent) => cb(payload)
    ipcRenderer.on(IPC.PANEL_DONE, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_DONE, handler)
  },
  onError: (cb: (event: ErrorEvent) => void): (() => void) => {
    const handler = (_: unknown, payload: ErrorEvent) => cb(payload)
    ipcRenderer.on(IPC.PANEL_ERROR, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_ERROR, handler)
  },
  onPageStatus: (cb: (status: PageStatus) => void): (() => void) => {
    const handler = (_: unknown, payload: PageStatus) => cb(payload)
    ipcRenderer.on(IPC.PANEL_PAGE_STATUS, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_PAGE_STATUS, handler)
  },
  onZoomChanged: (cb: (state: { factor: number; percent?: number }) => void): (() => void) => {
    const handler = (_: unknown, payload: { factor: number; percent?: number }) => cb(payload)
    ipcRenderer.on(IPC.PANEL_ZOOM_CHANGED, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_ZOOM_CHANGED, handler)
  },
  onDevModeChanged: (cb: (state: { enabled: boolean }) => void): (() => void) => {
    const handler = (_: unknown, payload: { enabled: boolean }) => cb(payload)
    ipcRenderer.on(IPC.PANEL_DEV_MODE_CHANGED, handler)
    return () => ipcRenderer.removeListener(IPC.PANEL_DEV_MODE_CHANGED, handler)
  }
})
