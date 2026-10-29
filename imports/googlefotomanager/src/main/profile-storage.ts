import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { app, type WebContents } from 'electron'

export interface WebStorageSnapshot {
  local: Record<string, string>
  session: Record<string, string>
}

function profileDataDir(): string {
  const dir = join(app.getPath('userData'), 'profile-data')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function storagePath(profileId: string): string {
  return join(profileDataDir(), `${profileId}-storage.json`)
}

export async function exportWebStorage(webContents: WebContents): Promise<WebStorageSnapshot> {
  return webContents.executeJavaScript(`({
    local: Object.fromEntries(
      Object.keys(localStorage).map((k) => [k, localStorage.getItem(k)])
    ),
    session: Object.fromEntries(
      Object.keys(sessionStorage).map((k) => [k, sessionStorage.getItem(k)])
    )
  })`)
}

export async function importWebStorage(
  webContents: WebContents,
  snapshot: WebStorageSnapshot
): Promise<void> {
  const payload = JSON.stringify(snapshot)
  await webContents.executeJavaScript(`
    (function(data) {
      for (const [k, v] of Object.entries(data.local || {})) {
        if (v != null) localStorage.setItem(k, v);
      }
      for (const [k, v] of Object.entries(data.session || {})) {
        if (v != null) sessionStorage.setItem(k, v);
      }
    })(${payload})
  `)
}

export function saveProfileStorage(profileId: string, snapshot: WebStorageSnapshot): void {
  writeFileSync(storagePath(profileId), JSON.stringify(snapshot), 'utf-8')
}

export function loadProfileStorage(profileId: string): WebStorageSnapshot | null {
  const path = storagePath(profileId)
  if (!existsSync(path)) return null
  try {
    return JSON.parse(readFileSync(path, 'utf-8')) as WebStorageSnapshot
  } catch {
    return null
  }
}

export function deleteProfileStorage(profileId: string): void {
  const path = storagePath(profileId)
  if (existsSync(path)) unlinkSync(path)
}
