import type { DownloadItem, Event, Session } from 'electron'

export const DEFAULT_DOWNLOAD_WAIT_MS = 15 * 60 * 1000

export type DownloadProgressInfo = {
  receivedBytes: number
  totalBytes: number
  percent: number
}

export type WaitForSessionDownloadOptions = {
  onProgress?: (info: DownloadProgressInfo) => void
  timeoutMs?: number
}

export type WaitForSessionDownloadResult = {
  ok: boolean
  step?: string
  state?: string
}

export type SessionDownloadWait = {
  promise: Promise<WaitForSessionDownloadResult>
  cancel: (result?: WaitForSessionDownloadResult) => void
}

export function downloadPercent(receivedBytes: number, totalBytes: number): number {
  if (totalBytes <= 0) return 0
  return Math.min(100, Math.max(0, (receivedBytes / totalBytes) * 100))
}

export function waitForSessionDownload(
  session: Session,
  options: WaitForSessionDownloadOptions = {}
): SessionDownloadWait {
  const timeoutMs = options.timeoutMs ?? DEFAULT_DOWNLOAD_WAIT_MS
  let settled = false
  let activeItem: DownloadItem | null = null
  let timeoutId: ReturnType<typeof setTimeout> | null = null
  let finish!: (result: WaitForSessionDownloadResult) => void

  const cleanup = (): void => {
    session.removeListener('will-download', onWillDownload)
    if (activeItem) {
      activeItem.removeListener('updated', onUpdated)
      activeItem.removeListener('done', onDone)
      activeItem = null
    }
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
  }

  const promise = new Promise<WaitForSessionDownloadResult>((resolve) => {
    finish = (result) => {
      if (settled) return
      settled = true
      cleanup()
      resolve(result)
    }
  })

  const emitProgress = (item: DownloadItem): void => {
    const receivedBytes = item.getReceivedBytes()
    const totalBytes = item.getTotalBytes()
    options.onProgress?.({
      receivedBytes,
      totalBytes,
      percent: downloadPercent(receivedBytes, totalBytes)
    })
  }

  const onUpdated = (): void => {
    if (activeItem) emitProgress(activeItem)
  }

  const onDone = (_event: Event, state: string): void => {
    if (activeItem) emitProgress(activeItem)
    if (state === 'completed') {
      finish({ ok: true, state })
      return
    }
    finish({ ok: false, step: 'download-failed', state })
  }

  const onWillDownload = (_event: Event, item: DownloadItem): void => {
    if (settled || activeItem) return
    activeItem = item
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    emitProgress(item)
    item.on('updated', onUpdated)
    item.once('done', onDone)
  }

  session.on('will-download', onWillDownload)

  timeoutId = setTimeout(() => {
    finish({ ok: false, step: 'download-timeout' })
  }, timeoutMs)

  return {
    promise,
    cancel: (result = { ok: false, step: 'cancelled' }) => finish(result)
  }
}
