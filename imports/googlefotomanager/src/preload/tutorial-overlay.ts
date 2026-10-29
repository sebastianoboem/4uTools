import { contextBridge, ipcRenderer } from 'electron'
import { IPC } from '../shared/ipc-channels'
import type { TutorialRenderPayload } from '../shared/tutorial-types'

contextBridge.exposeInMainWorld('tutorialOverlayApi', {
  action: (type: 'skip' | 'next', dismissForever: boolean): void => {
    ipcRenderer.send(IPC.TUTORIAL_ACTION, { type, dismissForever })
  },
  onRender: (cb: (payload: TutorialRenderPayload) => void): (() => void) => {
    const handler = (_: unknown, payload: TutorialRenderPayload) => {
      document.body.classList.toggle('is-intro', payload.mode === 'intro')
      cb(payload)
    }
    ipcRenderer.on(IPC.TUTORIAL_RENDER, handler)
    return () => ipcRenderer.removeListener(IPC.TUTORIAL_RENDER, handler)
  }
})
