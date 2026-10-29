import { contextBridge, ipcRenderer } from 'electron'
import { IPC } from '../shared/ipc-channels'
import { engine } from '../engine/selection-engine'
import type { SelectionParams } from '../shared/types'

engine.setCallbacks({
  onProgress: (event) => ipcRenderer.send(IPC.PHOTOS_PROGRESS, event),
  onLog: (event) => ipcRenderer.send(IPC.PHOTOS_LOG, event),
  onDone: (event) => ipcRenderer.send(IPC.PHOTOS_DONE, event),
  onError: (event) => ipcRenderer.send(IPC.PHOTOS_ERROR, event),
  onTriggerDownload: () => ipcRenderer.invoke(IPC.PHOTOS_TRIGGER_DOWNLOAD)
})

;(window as unknown as { __googleFotoEngine: typeof engine }).__googleFotoEngine = engine

ipcRenderer.on(IPC.PHOTOS_START, (_e, params: SelectionParams) => {
  void engine.start(params)
})

ipcRenderer.on(IPC.PHOTOS_DELETE, (_e, params: SelectionParams) => {
  void engine.startAndDelete(params)
})

ipcRenderer.on(IPC.PHOTOS_PAUSE, () => engine.pause())
ipcRenderer.on(IPC.PHOTOS_RESUME, () => engine.resume())
ipcRenderer.on(IPC.PHOTOS_STOP, () => {
  void engine.stop()
})
ipcRenderer.on(IPC.PHOTOS_DESELECT, () => {
  void engine.deselect()
})

ipcRenderer.on(IPC.PHOTOS_REDIAGNOSE, () => {
  ipcRenderer.send(IPC.PHOTOS_ERROR, {
    message: 'Diagnostica aggiornata',
    diagnostics: engine.diagnose()
  })
})

function emitPageStatus(): void {
  ipcRenderer.send(IPC.PHOTOS_PAGE_STATUS, engine.getPageStatus())
}

window.addEventListener('DOMContentLoaded', emitPageStatus)
window.addEventListener('load', emitPageStatus)

contextBridge.exposeInMainWorld('photosApi', {
  getPageStatus: () => engine.getPageStatus(),
  diagnose: () => engine.diagnose()
})
