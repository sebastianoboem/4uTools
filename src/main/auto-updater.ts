import { app, BrowserWindow, dialog } from 'electron'
import { autoUpdater } from 'electron-updater'
import { prepareAppQuit } from './app-quit'
import {
  GITHUB_UPDATE_FEED,
  genericFeed,
  localUpdateFeedUrl
} from '../shared/update-feeds'

const UPDATE_HTML = `<!DOCTYPE html>
<html lang="it">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Aggiornamento</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #1a1a1a;
      color: #f0f0f0;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
    }
    .card { width: 100%; max-width: 360px; text-align: center; }
    h1 { font-size: 1.1rem; font-weight: 600; margin-bottom: 8px; }
    p { font-size: 0.9rem; color: #b8b8b8; line-height: 1.45; margin-bottom: 16px; }
    progress {
      width: 100%;
      height: 8px;
      border: none;
      border-radius: 999px;
      overflow: hidden;
      background: #333;
    }
    progress::-webkit-progress-bar { background: #333; }
    progress::-webkit-progress-value { background: #4a9eff; }
    .pct { margin-top: 8px; font-size: 0.85rem; color: #888; }
  </style>
</head>
<body>
  <div class="card">
    <h1 id="title">Controllo aggiornamenti…</h1>
    <p id="message">Connessione in corso.</p>
    <progress id="bar" max="100" value="0" hidden></progress>
    <div id="pct" class="pct" hidden></div>
  </div>
  <script>
    window.updateUi = (payload) => {
      const title = document.getElementById('title');
      const message = document.getElementById('message');
      const bar = document.getElementById('bar');
      const pct = document.getElementById('pct');
      if (payload.title) title.textContent = payload.title;
      if (payload.message) message.textContent = payload.message;
      if (typeof payload.percent === 'number') {
        bar.hidden = false;
        pct.hidden = false;
        bar.value = payload.percent;
        pct.textContent = Math.round(payload.percent) + '%';
      }
    };
  </script>
</body>
</html>`

let updateWindow: BrowserWindow | null = null

export function shouldRunStartupUpdateCheck(): boolean {
  return app.isPackaged
}

function setUpdateUi(payload: {
  title?: string
  message?: string
  percent?: number
}): void {
  if (!updateWindow || updateWindow.isDestroyed()) return
  void updateWindow.webContents
    .executeJavaScript(`window.updateUi?.(${JSON.stringify(payload)})`)
    .catch(() => {})
}

function createUpdateWindow(): Promise<BrowserWindow> {
  if (updateWindow && !updateWindow.isDestroyed()) {
    return Promise.resolve(updateWindow)
  }

  updateWindow = new BrowserWindow({
    width: 420,
    height: 220,
    resizable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    title: 'Aggiornamento — Google Foto Manager',
    show: false,
    autoHideMenuBar: true,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true
    }
  })

  updateWindow.on('closed', () => {
    updateWindow = null
  })

  return new Promise((resolve, reject) => {
    const win = updateWindow!
    win.webContents.once('did-finish-load', () => {
      win.show()
      resolve(win)
    })
    win.webContents.once('did-fail-load', (_event, _code, description) => {
      reject(new Error(description))
    })
    void win.loadURL(
      `data:text/html;charset=utf-8,${encodeURIComponent(UPDATE_HTML)}`
    )
  })
}

function closeUpdateWindow(): void {
  if (!updateWindow || updateWindow.isDestroyed()) return
  updateWindow.close()
  updateWindow = null
}

function configureAutoUpdater(): void {
  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = false
  autoUpdater.allowDowngrade = false
}

function applyUpdateFeed(): void {
  const localUrl = localUpdateFeedUrl()
  if (localUrl) {
    autoUpdater.setFeedURL(genericFeed(localUrl))
    return
  }
  autoUpdater.setFeedURL(GITHUB_UPDATE_FEED)
}

async function promptRestartToInstall(version: string): Promise<never> {
  setUpdateUi({
    title: 'Aggiornamento pronto',
    message: `La versione ${version} è stata scaricata. L'app verrà riavviata per completare l'installazione.`
  })

  await dialog.showMessageBox({
    type: 'info',
    title: 'Aggiornamento pronto',
    message: `La versione ${version} è pronta.`,
    detail: 'L\'app verrà chiusa e riavviata per applicare l\'aggiornamento.',
    buttons: ['Riavvia ora'],
    defaultId: 0,
    cancelId: 0,
    noLink: true
  })

  prepareAppQuit()
  closeUpdateWindow()
  app.removeAllListeners('activate')
  autoUpdater.quitAndInstall(true, true)
  return new Promise(() => {})
}

export async function runStartupUpdateCheck(): Promise<void> {
  if (!shouldRunStartupUpdateCheck()) return

  configureAutoUpdater()
  autoUpdater.removeAllListeners()

  try {
    await createUpdateWindow()
  } catch (error) {
    console.warn('[auto-updater]', error)
    return
  }

  return new Promise((resolve) => {
    let settled = false

    const finish = (): void => {
      if (settled) return
      settled = true
      closeUpdateWindow()
      resolve()
    }

    const localUrl = localUpdateFeedUrl()
    setUpdateUi({
      title: 'Controllo aggiornamenti…',
      message: localUrl
        ? 'Verifica aggiornamenti (feed locale)…'
        : 'Verifica aggiornamenti su GitHub…'
    })

    applyUpdateFeed()

    autoUpdater.on('checking-for-update', () => {
      setUpdateUi({
        title: 'Controllo aggiornamenti…',
        message: localUrl
          ? 'Verifica aggiornamenti (feed locale)…'
          : 'Verifica aggiornamenti su GitHub…'
      })
    })

    autoUpdater.on('update-not-available', () => {
      finish()
    })

    autoUpdater.on('download-progress', (progress) => {
      setUpdateUi({
        title: 'Download aggiornamento…',
        message: 'Scaricamento della nuova versione in corso.',
        percent: progress.percent
      })
    })

    autoUpdater.on('update-downloaded', (info) => {
      void promptRestartToInstall(info.version)
    })

    autoUpdater.on('update-available', (info) => {
      setUpdateUi({
        title: 'Aggiornamento disponibile',
        message: `Versione ${info.version} trovata. Download in corso…`
      })
    })

    autoUpdater.on('error', (error) => {
      console.warn('[auto-updater]', error.message)
      finish()
    })

    void autoUpdater.checkForUpdates().catch((error: Error) => {
      console.warn('[auto-updater]', error.message)
      finish()
    })
  })
}
