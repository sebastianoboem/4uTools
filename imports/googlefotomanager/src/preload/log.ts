import { contextBridge, ipcRenderer } from 'electron'
import { IPC } from '../shared/ipc-channels'
import type { LogEvent } from '../shared/types'

contextBridge.exposeInMainWorld('logApi', {
  clear: (): void => ipcRenderer.send(IPC.LOG_REQUEST_CLEAR),
  getHistory: (): Promise<LogEvent[]> => ipcRenderer.invoke(IPC.LOG_GET_HISTORY),
  onAppend: (cb: (event: LogEvent) => void): (() => void) => {
    const handler = (_: unknown, payload: LogEvent) => cb(payload)
    ipcRenderer.on(IPC.LOG_APPEND, handler)
    return () => ipcRenderer.removeListener(IPC.LOG_APPEND, handler)
  },
  onClear: (cb: () => void): (() => void) => {
    const handler = () => cb()
    ipcRenderer.on(IPC.LOG_CLEAR, handler)
    return () => ipcRenderer.removeListener(IPC.LOG_CLEAR, handler)
  }
})
