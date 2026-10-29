import type {
  DoneEvent,
  ErrorDiagnostics,
  ErrorEvent,
  LogEvent,
  PageStatus,
  ProgressEvent,
  SelectionParams
} from './types'

export const IPC = {
  // Panel → Main
  PANEL_GET_SETTINGS: 'panel:get-settings',
  PANEL_SAVE_SETTINGS: 'panel:save-settings',
  PANEL_RESET_SETTINGS: 'panel:reset-settings',
  PANEL_START: 'panel:start',
  PANEL_PAUSE: 'panel:pause',
  PANEL_RESUME: 'panel:resume',
  PANEL_STOP: 'panel:stop',
  PANEL_DELETE: 'panel:delete',
  PANEL_DESELECT: 'panel:deselect',
  PANEL_OPEN_LOG: 'panel:open-log',
  PANEL_CLOSE_LOG: 'panel:close-log',
  PANEL_REDIAGNOSE: 'panel:rediagnose',
  PANEL_GET_PAGE_STATUS: 'panel:get-page-status',
  PANEL_CHROME_LOGIN: 'panel:chrome-login',
  PANEL_LIST_PROFILES: 'panel:list-profiles',
  PANEL_SWITCH_PROFILE: 'panel:switch-profile',
  PANEL_SAVE_PROFILE: 'panel:save-profile',
  PANEL_DELETE_PROFILE: 'panel:delete-profile',
  PANEL_GET_ZOOM: 'panel:get-zoom',
  PANEL_ZOOM_STEP: 'panel:zoom-step',
  PANEL_ZOOM_RESET: 'panel:zoom-reset',
  PANEL_LOG_LINE: 'panel:log-line',
  PANEL_LOG_CLEAR: 'panel:log-clear',

  // Main → Panel (events)
  PANEL_PROGRESS: 'panel:progress',
  PANEL_DONE: 'panel:done',
  PANEL_ERROR: 'panel:error',
  PANEL_PAGE_STATUS: 'panel:page-status',
  PANEL_ZOOM_CHANGED: 'panel:zoom-changed',
  PANEL_GET_DEV_MODE: 'panel:get-dev-mode',
  PANEL_GET_APP_VERSION: 'panel:get-app-version',
  PANEL_DEV_MODE_CHANGED: 'panel:dev-mode-changed',
  PANEL_GET_TUTORIAL_COMPLETED: 'panel:get-tutorial-completed',
  PANEL_SET_TUTORIAL_COMPLETED: 'panel:set-tutorial-completed',

  TUTORIAL_SHOW: 'tutorial:show',
  TUTORIAL_HIDE: 'tutorial:hide',
  TUTORIAL_RENDER_REQUEST: 'tutorial:render-request',
  TUTORIAL_RENDER: 'tutorial:render',
  TUTORIAL_ACTION: 'tutorial:action',
  TUTORIAL_REPOSITION: 'tutorial:reposition',

  // Main → Log
  LOG_APPEND: 'log:append',
  LOG_CLEAR: 'log:clear',

  // Log → Main
  LOG_REQUEST_CLEAR: 'log:request-clear',
  LOG_GET_HISTORY: 'log:get-history',

  // Main ↔ Photos
  PHOTOS_START: 'photos:start',
  PHOTOS_PAUSE: 'photos:pause',
  PHOTOS_RESUME: 'photos:resume',
  PHOTOS_STOP: 'photos:stop',
  PHOTOS_DELETE: 'photos:delete',
  PHOTOS_DESELECT: 'photos:deselect',
  PHOTOS_REDIAGNOSE: 'photos:rediagnose',
  PHOTOS_GET_STATUS: 'photos:get-status',

  PHOTOS_PROGRESS: 'photos:progress',
  PHOTOS_LOG: 'photos:log',
  PHOTOS_DONE: 'photos:done',
  PHOTOS_ERROR: 'photos:error',
  PHOTOS_PAGE_STATUS: 'photos:page-status',
  PHOTOS_TRIGGER_DOWNLOAD: 'photos:trigger-download'
} as const

export type IpcChannel = (typeof IPC)[keyof typeof IPC]

export type PanelGetSettingsResponse = SelectionParams
export type PanelSaveSettingsPayload = SelectionParams
export type PanelStartPayload = SelectionParams
export type PanelProgressPayload = ProgressEvent
export type PanelDonePayload = DoneEvent
export type PanelErrorPayload = ErrorEvent
export type PanelPageStatusPayload = PageStatus
export type LogAppendPayload = LogEvent
export type PhotosStartPayload = SelectionParams
export type PhotosProgressPayload = ProgressEvent
export type PhotosLogPayload = LogEvent
export type PhotosDonePayload = DoneEvent
export type PhotosErrorPayload = ErrorEvent
export type PhotosPageStatusPayload = PageStatus
export type PhotosRediagnoseResponse = ErrorDiagnostics
