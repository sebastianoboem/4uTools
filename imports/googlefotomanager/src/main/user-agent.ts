import type { Session } from 'electron'

const CHROME_MAJOR = '136'

function platformChromeUa(): string {
  switch (process.platform) {
    case 'win32':
      return `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${CHROME_MAJOR}.0.0.0 Safari/537.36`
    case 'linux':
      return `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${CHROME_MAJOR}.0.0.0 Safari/537.36`
    default:
      return `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${CHROME_MAJOR}.0.0.0 Safari/537.36`
  }
}

export function getCleanUserAgent(fallback?: string): string {
  const raw = fallback ?? platformChromeUa()
  return raw
    .replace(/\s*Electron\/[\d.]+\s*/gi, ' ')
    .replace(/\s*google-foto-manager\/[\d.]+\s*/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

export function getChromeClientHints(): {
  secChUa: string
  secChUaMobile: string
  secChUaPlatform: string
} {
  const platform =
    process.platform === 'win32'
      ? '"Windows"'
      : process.platform === 'linux'
        ? '"Linux"'
        : '"macOS"'

  return {
    secChUa: `"Chromium";v="${CHROME_MAJOR}", "Google Chrome";v="${CHROME_MAJOR}", "Not.A/Brand";v="99"`,
    secChUaMobile: '?0',
    secChUaPlatform: platform
  }
}

/** Force Chrome-like UA + client hints on every request (XHR/fetch included). */
export function setupSessionRequestHeaders(ses: Session, ua: string): void {
  const hints = getChromeClientHints()

  ses.webRequest.onBeforeSendHeaders((details, callback) => {
    const requestHeaders = { ...details.requestHeaders }
    requestHeaders['User-Agent'] = ua
    requestHeaders['Sec-CH-UA'] = hints.secChUa
    requestHeaders['Sec-CH-UA-Mobile'] = hints.secChUaMobile
    requestHeaders['Sec-CH-UA-Platform'] = hints.secChUaPlatform
    callback({ requestHeaders })
  })
}

export function applyCleanUserAgent(app: Electron.App): string {
  const ua = getCleanUserAgent(app.userAgentFallback || platformChromeUa())
  app.userAgentFallback = ua
  return ua
}
