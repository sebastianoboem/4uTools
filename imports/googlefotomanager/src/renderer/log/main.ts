import type { LogEvent } from '../../shared/types'

declare global {
  interface Window {
    logApi?: {
      clear: () => void
      getHistory: () => Promise<LogEvent[]>
      onAppend: (cb: (event: LogEvent) => void) => () => void
      onClear: (cb: () => void) => () => void
    }
  }
}

const list = document.getElementById('log-list') as HTMLUListElement
const btnClear = document.getElementById('btn-clear') as HTMLButtonElement

function formatTs(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleTimeString('it-IT', { hour12: false })
}

function appendEntry(event: LogEvent): void {
  const li = document.createElement('li')
  li.className = `level-${event.level}`
  li.textContent = `[${formatTs(event.ts)}] [${event.level}] ${event.msg}`
  list.appendChild(li)
  list.scrollTop = list.scrollHeight
}

async function init(): Promise<void> {
  if (!window.logApi) {
    list.innerHTML = ''
    const li = document.createElement('li')
    li.className = 'level-errore'
    li.textContent = 'Preload log non disponibile. Riavvia l\'app.'
    list.appendChild(li)
    return
  }

  btnClear.addEventListener('click', () => window.logApi?.clear())
  window.logApi.onAppend(appendEntry)
  window.logApi.onClear(() => {
    list.innerHTML = ''
  })

  try {
    const history = await window.logApi.getHistory()
    for (const event of history) appendEntry(event)
  } catch {
    /* history optional */
  }
}

void init()
