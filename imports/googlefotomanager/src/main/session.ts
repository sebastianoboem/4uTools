import { session as electronSession, type Session } from 'electron'
import { getCleanUserAgent, setupSessionRequestHeaders } from './user-agent'
export const EPHEMERAL_PARTITION = 'google-foto-ephemeral'

export function partitionForProfile(profileId: string | null): string {
  return profileId ? `persist:gfm-${profileId}` : EPHEMERAL_PARTITION
}

export function createSessionForProfile(profileId: string | null): Session {
  const partition = partitionForProfile(profileId)
  const ses = electronSession.fromPartition(partition, { cache: profileId !== null })
  const ua = getCleanUserAgent()
  ses.setUserAgent(ua)
  setupSessionRequestHeaders(ses, ua)
  return ses
}

export function createEphemeralSession(): Session {
  return createSessionForProfile(null)
}

export async function copySessionCookies(source: Session, destination: Session): Promise<number> {
  const cookies = await source.cookies.get({})
  let copied = 0

  for (const cookie of cookies) {
    if (!cookie.name || !cookie.domain) continue

    const host = cookie.domain.replace(/^\./, '')
    const domain = cookie.domain.startsWith('.') ? cookie.domain : `.${cookie.domain}`
    const scheme = cookie.secure ? 'https' : 'http'
    const path = cookie.path || '/'
    const url = `${scheme}://${host}${path}`

    try {
      await destination.cookies.set({
        url,
        name: cookie.name,
        value: cookie.value,
        domain,
        path,
        secure: cookie.secure,
        httpOnly: cookie.httpOnly,
        expirationDate: cookie.expirationDate,
        sameSite: cookie.sameSite
      })
      copied++
    } catch {
      // skip invalid cookie entries
    }
  }

  await destination.cookies.flushStore().catch(() => {})
  return copied
}

export function setupSessionPermissions(ses: Electron.Session): void {
  ses.setPermissionRequestHandler((_wc, _permission, callback) => {
    callback(true)
  })
}
