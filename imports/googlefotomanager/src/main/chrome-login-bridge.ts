import { existsSync } from 'node:fs'
import { mkdtemp, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import puppeteer, { type Browser, type Cookie, type Page } from 'puppeteer-core'
import type { Session } from 'electron'
import { isPhotosGoogleUrl } from '../engine/selectors'

const PHOTOS_URL = 'https://photos.google.com/'
/** Puppeteer default args that make Google reject sign-in. */
const PUPPETEER_AUTOMATION_ARGS = ['--enable-automation'] as const
const LOGIN_URLS = [
  'https://photos.google.com/',
  'https://accounts.google.com/',
  'https://www.google.com/'
]

export type LoginBridgeResult =
  | { ok: true; cookieCount: number }
  | { ok: false; message: string }

export type LoginBrowserKind = 'chrome' | 'edge'

export interface LoginBrowserTarget {
  kind: LoginBrowserKind
  displayName: string
  executablePath?: string
}

export function getEdgeExecutableCandidates(): string[] {
  switch (process.platform) {
    case 'win32':
      return [
        'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
        'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe'
      ]
    case 'darwin':
      return ['/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge']
    default:
      return [
        '/usr/bin/microsoft-edge',
        '/usr/bin/microsoft-edge-stable',
        '/usr/bin/microsoft-edge-dev'
      ]
  }
}

export function findEdgeExecutable(): string | null {
  for (const candidate of getEdgeExecutableCandidates()) {
    if (existsSync(candidate)) return candidate
  }
  return null
}

export function findChromeExecutable(): string | null {
  try {
    const path = puppeteer.executablePath('chrome')
    return existsSync(path) ? path : null
  } catch {
    return null
  }
}

/** Prefer Chrome; fall back to Microsoft Edge (preinstalled on Windows). */
export function resolveLoginBrowser(): LoginBrowserTarget {
  if (findChromeExecutable()) {
    return { kind: 'chrome', displayName: 'Chrome' }
  }
  const edgePath = findEdgeExecutable()
  if (edgePath) {
    return { kind: 'edge', displayName: 'Microsoft Edge', executablePath: edgePath }
  }
  throw new Error(
    'Né Google Chrome né Microsoft Edge trovati. Installa uno dei due per il login.'
  )
}

/** Launch options so the browser is not flagged as automated (Google blocks sign-in otherwise). */
export function buildLoginLaunchOptions(
  userDataDir: string,
  target: LoginBrowserTarget
): Parameters<typeof puppeteer.launch>[0] {
  const common = {
    headless: false,
    userDataDir,
    ignoreDefaultArgs: [...PUPPETEER_AUTOMATION_ARGS],
    args: [
      '--disable-blink-features=AutomationControlled',
      '--no-first-run',
      '--no-default-browser-check'
    ]
  } as const

  if (target.kind === 'chrome') {
    return { ...common, channel: 'chrome' }
  }
  return { ...common, executablePath: target.executablePath }
}

/** @deprecated Use buildLoginLaunchOptions */
export function buildChromeLoginLaunchOptions(userDataDir: string): Parameters<
  typeof puppeteer.launch
>[0] {
  return buildLoginLaunchOptions(userDataDir, { kind: 'chrome', displayName: 'Chrome' })
}

async function applyStealthToPage(page: Page): Promise<void> {
  await page.evaluateOnNewDocument(() => {
    Object.defineProperty(navigator, 'webdriver', {
      get: () => undefined,
      configurable: true
    })
  })
}

/** Apre Chrome o Edge per il login; l'utente attende il sync automatico. */
export async function runChromeLoginBridge(
  targetSession: Session,
  onStatus?: (msg: string) => void
): Promise<LoginBridgeResult> {
  const userDataDir = await mkdtemp(join(tmpdir(), 'gfm-login-'))
  let browser: Browser | null = null
  let browserTarget: LoginBrowserTarget

  try {
    browserTarget = resolveLoginBrowser()
    onStatus?.(`Apertura di ${browserTarget.displayName} per il login…`)
    browser = await puppeteer.launch(buildLoginLaunchOptions(userDataDir, browserTarget))

    const pages = await browser.pages()
    const page = pages[0] ?? (await browser.newPage())
    await applyStealthToPage(page)
    for (const extra of pages) {
      if (extra !== page) await extra.close().catch(() => {})
    }

    onStatus?.(
      `Accedi al tuo account Google in ${browserTarget.displayName}, poi torna qui.`
    )
    await page.goto(PHOTOS_URL, { waitUntil: 'domcontentloaded', timeout: 120_000 })

    const loggedIn = await waitForPhotosLogin(page, 300_000, browserTarget.displayName, onStatus)
    if (!loggedIn) {
      return {
        ok: false,
        message: 'Login non completato in tempo. Riprova «Accedi con Chrome».'
      }
    }

    onStatus?.('Importazione sessione nell\'app…')
    const count = await importCookiesFromPage(page, targetSession)
    if (count === 0) {
      return {
        ok: false,
        message: `Nessun cookie importato. Verifica di essere loggato in ${browserTarget.displayName}.`
      }
    }

    return { ok: true, cookieCount: count }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    return { ok: false, message: `Login browser fallito: ${message}` }
  } finally {
    if (browser) await browser.close().catch(() => {})
    await rm(userDataDir, { recursive: true, force: true }).catch(() => {})
  }
}

async function waitForPhotosLogin(
  page: Page,
  timeoutMs: number,
  browserName: string,
  onStatus?: (msg: string) => void
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const url = page.url()
    if (url.includes('/signin/rejected') || url.includes('signin/rejected')) {
      onStatus?.(
        `Google ha rifiutato il login (browser automatizzato). Chiudi ${browserName} e riprova.`
      )
      return false
    }
    if (isPhotosGoogleUrl(url)) {
      const blocked = await page.evaluate(() => {
        const text = document.body?.innerText ?? ''
        return (
          text.includes('non essere sicur') ||
          text.includes('may not be secure') ||
          text.includes('Impossibile eseguire l\'accesso')
        )
      })
      if (!blocked) {
        const hasMain = await page.evaluate(
          () => document.querySelector('[role="main"]') !== null
        )
        if (hasMain) return true
      }
    }
    onStatus?.(`In attesa del login su photos.google.com in ${browserName}…`)
    await new Promise((r) => setTimeout(r, 2000))
  }
  return false
}

async function importCookiesFromPage(page: Page, session: Session): Promise<number> {
  const collected = new Map<string, Cookie>()
  for (const url of LOGIN_URLS) {
    const list = await page.cookies(url)
    for (const c of list) {
      collected.set(`${c.name}@${c.domain}`, c)
    }
  }

  let count = 0
  for (const c of collected.values()) {
    const domain = c.domain.startsWith('.') ? c.domain : `.${c.domain}`
    const host = c.domain.replace(/^\./, '')
    const scheme = c.secure ? 'https' : 'http'
    const url = `${scheme}://${host}${c.path || '/'}`

    try {
      await session.cookies.set({
        url,
        name: c.name,
        value: c.value,
        domain,
        path: c.path || '/',
        secure: c.secure,
        httpOnly: c.httpOnly,
        expirationDate: c.expires > 0 ? c.expires : undefined,
        sameSite:
          c.sameSite === 'Strict'
            ? 'strict'
            : c.sameSite === 'Lax'
              ? 'lax'
              : c.sameSite === 'None'
                ? 'no_restriction'
                : 'unspecified'
      })
      count++
    } catch {
      /* skip invalid cookie */
    }
  }
  return count
}
