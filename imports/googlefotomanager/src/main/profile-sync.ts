import type { Session } from 'electron'
import {
  applyProfileCookies,
  countAuthCookies,
  loadProfileCookies,
  removeGoogleCookies,
  sidFromCookies
} from './profile-cookies'

const NON_COOKIE_STORAGES = [
  'indexdb',
  'cachestorage',
  'serviceworkers',
  'localstorage',
  'shadercache'
] as const

export async function applyProfileToEphemeralSession(
  session: Session,
  profileId: string | null
): Promise<'cleared' | 'skipped' | 'imported'> {
  if (!profileId) {
    await session.clearStorageData({ storages: ['cookies', ...NON_COOKIE_STORAGES] })
    return 'cleared'
  }

  const snapshot = loadProfileCookies(profileId)
  const current = await session.cookies.get({})
  const currentSid = sidFromCookies(current)
  const snapshotSid = snapshot ? sidFromCookies(snapshot) : undefined
  const currentAuthCount = countAuthCookies(current)

  // Already authenticated as the same account: don't touch the working session.
  if (currentAuthCount >= 3 && snapshotSid && currentSid === snapshotSid) {
    return 'skipped'
  }

  const needsGoogleCookieReset =
    currentAuthCount < 3 || Boolean(snapshotSid && currentSid && currentSid !== snapshotSid)

  if (needsGoogleCookieReset) {
    await removeGoogleCookies(session)
  }

  await session.clearStorageData({ storages: [...NON_COOKIE_STORAGES] })
  await session.clearCache()
  await applyProfileCookies(session, profileId)
  return 'imported'
}
