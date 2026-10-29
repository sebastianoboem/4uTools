import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { randomUUID } from 'node:crypto'
import { app } from 'electron'
import type { SavedProfile, ProfileStoreData } from '../shared/types'

const PROFILES_FILE = 'profiles.json'

export type { ProfileStoreData }

function storePath(): string {
  const dir = app.getPath('userData')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return join(dir, PROFILES_FILE)
}

function defaultStore(): ProfileStoreData {
  return { profiles: [], activeProfileId: null }
}

export function loadProfileStore(): ProfileStoreData {
  const path = storePath()
  if (!existsSync(path)) return defaultStore()
  try {
    const raw = JSON.parse(readFileSync(path, 'utf-8')) as Partial<ProfileStoreData>
    return {
      profiles: Array.isArray(raw.profiles) ? raw.profiles : [],
      activeProfileId: typeof raw.activeProfileId === 'string' ? raw.activeProfileId : null
    }
  } catch {
    return defaultStore()
  }
}

function saveProfileStore(data: ProfileStoreData): ProfileStoreData {
  writeFileSync(storePath(), JSON.stringify(data, null, 2), 'utf-8')
  return data
}

export function listProfiles(): ProfileStoreData {
  return loadProfileStore()
}

export function getActiveProfileId(): string | null {
  return loadProfileStore().activeProfileId
}

export function createProfile(name: string): SavedProfile {
  const store = loadProfileStore()
  const profile: SavedProfile = {
    id: randomUUID(),
    name: name.trim() || 'Profilo senza nome',
    createdAt: Date.now()
  }
  store.profiles.push(profile)
  saveProfileStore(store)
  return profile
}

export function deleteProfile(profileId: string): boolean {
  const store = loadProfileStore()
  const before = store.profiles.length
  store.profiles = store.profiles.filter((p) => p.id !== profileId)
  if (store.activeProfileId === profileId) store.activeProfileId = null
  saveProfileStore(store)
  return store.profiles.length < before
}

export function setActiveProfile(profileId: string | null): ProfileStoreData {
  const store = loadProfileStore()
  if (profileId !== null && !store.profiles.some((p) => p.id === profileId)) {
    throw new Error('Profilo non trovato.')
  }
  store.activeProfileId = profileId
  return saveProfileStore(store)
}

export function renameProfile(profileId: string, name: string): SavedProfile | null {
  const store = loadProfileStore()
  const profile = store.profiles.find((p) => p.id === profileId)
  if (!profile) return null
  profile.name = name.trim() || profile.name
  saveProfileStore(store)
  return profile
}
