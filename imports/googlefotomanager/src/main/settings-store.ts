import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { app } from 'electron'
import {
  clampParams,
  DEFAULT_SELECTION_PARAMS
} from '../shared/settings-defaults'
import type { SelectionParams } from '../shared/types'

const SETTINGS_FILE = 'settings.json'

function settingsPath(): string {
  const dir = app.getPath('userData')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return join(dir, SETTINGS_FILE)
}

export function loadSettings(): SelectionParams {
  const path = settingsPath()
  if (!existsSync(path)) return { ...DEFAULT_SELECTION_PARAMS }
  try {
    const raw = JSON.parse(readFileSync(path, 'utf-8')) as Partial<SelectionParams>
    return clampParams({ ...DEFAULT_SELECTION_PARAMS, ...raw })
  } catch {
    return { ...DEFAULT_SELECTION_PARAMS }
  }
}

export function saveSettings(params: SelectionParams): SelectionParams {
  const clamped = clampParams(params)
  writeFileSync(settingsPath(), JSON.stringify(clamped, null, 2), 'utf-8')
  return clamped
}

export function resetSettings(): SelectionParams {
  return saveSettings({ ...DEFAULT_SELECTION_PARAMS })
}

export function getSettingsFilePath(): string {
  return settingsPath()
}
