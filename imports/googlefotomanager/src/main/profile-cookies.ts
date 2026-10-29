import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import type { CookiesSetDetails, Session } from 'electron'
import { app } from 'electron'

export type CookieSnapshot = Electron.Cookie[]

const AUTH_COOKIE_NAMES = new Set([
  'SID',
  'HSID',
  'SSID',
  'APISID',
  'SAPISID',
  '__Secure-1PSID',
  '__Secure-3PSID',
  '__Secure-1PAPISID',
  '__Secure-3PAPISID'
])

export function countAuthCookies(cookies: CookieSnapshot): number {
  return cookies.filter((c) => AUTH_COOKIE_NAMES.has(c.name)).length
}

function profileDataDir(): string {
  const dir = join(app.getPath('userData'), 'profile-data')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function cookiesPath(profileId: string): string {
  return join(profileDataDir(), `${profileId}-cookies.json`)
}

export function saveProfileCookies(profileId: string, cookies: CookieSnapshot): void {
  writeFileSync(cookiesPath(profileId), JSON.stringify(cookies, null, 2), 'utf-8')
}

export function loadProfileCookies(profileId: string): CookieSnapshot | null {
  const path = cookiesPath(profileId)
  if (!existsSync(path)) return null
  try {
    const raw = JSON.parse(readFileSync(path, 'utf-8'))
    return Array.isArray(raw) ? (raw as CookieSnapshot) : null
  } catch {
    return null
  }
}

export function deleteProfileCookies(profileId: string): void {
  const path = cookiesPath(profileId)
  if (existsSync(path)) unlinkSync(path)
}

function cookieToSetDetails(cookie: Electron.Cookie): CookiesSetDetails | null {
  if (!cookie.name || !cookie.domain) return null

  const host = cookie.domain.replace(/^\./, '')
  const scheme = cookie.secure ? 'https' : 'http'
  const path = cookie.path || '/'
  const url = `${scheme}://${host}${path}`

  const details: CookiesSetDetails = {
    url,
    name: cookie.name,
    value: cookie.value,
    path,
    secure: cookie.secure,
    httpOnly: cookie.httpOnly,
    expirationDate: cookie.expirationDate,
    sameSite: cookie.sameSite
  }

  if (!cookie.name.startsWith('__Host-')) {
    details.domain = cookie.domain.startsWith('.') ? cookie.domain : `.${cookie.domain}`
  }

  return details
}

export async function importCookiesToSession(
  session: Session,
  cookies: CookieSnapshot
): Promise<number> {
  let imported = 0
  for (const cookie of cookies) {
    const details = cookieToSetDetails(cookie)
    if (!details) continue
    try {
      await session.cookies.set(details)
      imported++
    } catch {
      // retry host-only without domain prefix
      if (details.domain) {
        try {
          const { domain: _d, ...rest } = details
          await session.cookies.set(rest)
          imported++
        } catch {
          // skip
        }
      }
    }
  }
  await session.cookies.flushStore().catch(() => {})
  return imported
}

export function sidFromCookies(cookies: CookieSnapshot): string | undefined {
  return cookies.find((c) => c.name === 'SID')?.value
}

export async function removeGoogleCookies(session: Session): Promise<number> {
  const cookies = await session.cookies.get({})
  let removed = 0
  for (const cookie of cookies) {
    if (!cookie.domain?.includes('google')) continue
    const host = cookie.domain.replace(/^\./, '')
    const scheme = cookie.secure ? 'https' : 'http'
    const path = cookie.path || '/'
    const url = `${scheme}://${host}${path}`
    try {
      await session.cookies.remove(url, cookie.name)
      removed++
    } catch {
      // skip
    }
  }
  await session.cookies.flushStore().catch(() => {})
  return removed
}

export async function applyProfileCookies(session: Session, profileId: string): Promise<number> {
  const snapshot = loadProfileCookies(profileId)
  if (!snapshot?.length) {
    const existing = await session.cookies.get({})
    return existing.length
  }

  // Guard against corrupted/incomplete snapshots (e.g. saved before login completed).
  if (countAuthCookies(snapshot) < 3) {
    const existing = await session.cookies.get({})
    return existing.length
  }

  // CDN session cookies (OSID on *.usercontent.google.com) rotate server-side;
  // replaying stale ones makes thumbnails 403. Google regenerates them on load.
  const importable = snapshot.filter((c) => !c.domain?.includes('usercontent.google.com'))
  return importCookiesToSession(session, importable)
}
