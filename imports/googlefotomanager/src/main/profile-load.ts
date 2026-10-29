import type { WebContents } from 'electron'
import { importWebStorage, loadProfileStorage } from './profile-storage'
import { applyProfileToEphemeralSession } from './profile-sync'

const PHOTOS_URL = 'https://photos.google.com/'

export async function loadPhotosWithProfile(
  webContents: WebContents,
  profileId: string | null,
  cookieAction: 'cleared' | 'skipped' | 'imported'
): Promise<void> {
  const storageSnapshot = profileId ? loadProfileStorage(profileId) : null
  const localKeys = storageSnapshot ? Object.keys(storageSnapshot.local).length : 0
  const currentUrl = webContents.getURL()

  if (cookieAction === 'skipped' && currentUrl.includes('photos.google.com')) {
    webContents.reloadIgnoringCache()
    return
  }

  if (cookieAction === 'imported') {
    await webContents.session.clearCache()
  }

  if (storageSnapshot && localKeys > 0) {
    webContents.loadURL(PHOTOS_URL)
    webContents.once('dom-ready', () => {
      void (async () => {
        await importWebStorage(webContents, storageSnapshot)
        webContents.reloadIgnoringCache()
      })()
    })
  } else {
    webContents.loadURL(PHOTOS_URL)
  }
}

export async function activateProfileOnSession(
  session: Electron.Session,
  profileId: string | null
): Promise<'cleared' | 'skipped' | 'imported'> {
  return applyProfileToEphemeralSession(session, profileId)
}
