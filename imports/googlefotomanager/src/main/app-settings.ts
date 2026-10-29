import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { app } from 'electron'

export interface AppSettings {
  devMode: boolean
  tutorialCompleted: boolean
}

const SETTINGS_FILE = 'app-settings.json'

const DEFAULTS: AppSettings = {
  devMode: false,
  tutorialCompleted: false
}

function settingsPath(): string {
  const dir = app.getPath('userData')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return join(dir, SETTINGS_FILE)
}

export function loadAppSettings(): AppSettings {
  const path = settingsPath()
  if (!existsSync(path)) return { ...DEFAULTS }
  try {
    const raw = JSON.parse(readFileSync(path, 'utf-8')) as Partial<AppSettings>
    return { ...DEFAULTS, ...raw }
  } catch {
    return { ...DEFAULTS }
  }
}

export function saveAppSettings(settings: AppSettings): AppSettings {
  writeFileSync(settingsPath(), JSON.stringify(settings, null, 2), 'utf-8')
  return settings
}

export function getDevMode(): boolean {
  return loadAppSettings().devMode
}

export function setDevMode(enabled: boolean): AppSettings {
  return saveAppSettings({ ...loadAppSettings(), devMode: enabled })
}

export function getTutorialCompleted(): boolean {
  return loadAppSettings().tutorialCompleted
}

export function setTutorialCompleted(completed: boolean): AppSettings {
  return saveAppSettings({ ...loadAppSettings(), tutorialCompleted: completed })
}
