import { Menu, type MenuItem } from 'electron'
import { getDevMode, setDevMode } from './app-settings'

let devModeMenuItem: MenuItem | null = null
let onDevModeChange: ((enabled: boolean) => void) | null = null

export function setDevModeChangeHandler(handler: (enabled: boolean) => void): void {
  onDevModeChange = handler
}

export function updateDevModeMenuItem(checked: boolean): void {
  if (devModeMenuItem) devModeMenuItem.checked = checked
}

export function setupApplicationMenu(): void {
  const isMac = process.platform === 'darwin'

  const template: Electron.MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            label: appName(),
            submenu: [{ role: 'quit' as const }]
          }
        ]
      : []),
    {
      label: 'Help',
      role: isMac ? ('help' as const) : undefined,
      submenu: [
        {
          id: 'dev-mode',
          label: 'Dev Mode',
          type: 'checkbox',
          checked: getDevMode(),
          click: (menuItem) => {
            const enabled = menuItem.checked
            setDevMode(enabled)
            onDevModeChange?.(enabled)
          }
        }
      ]
    }
  ]

  const menu = Menu.buildFromTemplate(template)
  devModeMenuItem = menu.getMenuItemById('dev-mode')
  Menu.setApplicationMenu(menu)
}

function appName(): string {
  return 'Google Foto Manager'
}
