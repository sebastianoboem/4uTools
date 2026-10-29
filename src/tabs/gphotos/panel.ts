import type { EngineState, LogEvent, PageStatus, ProfileStoreData, ProgressEvent, SelectionParams } from "./engine/types";
import { PARAM_RANGES, PARAM_TOOLTIPS } from "./engine/settings-defaults";
import { api } from "./api";
import { attachTutorial } from "./tutorial-layer";

const form = document.getElementById('params-form') as HTMLFormElement
const actionsSection = document.getElementById('actions-section') as HTMLElement
const btnStart = document.getElementById('btn-start') as HTMLButtonElement
const btnDelete = document.getElementById('btn-delete') as HTMLButtonElement
const btnPause = document.getElementById('btn-pause') as HTMLButtonElement
const btnClearLog = document.getElementById('btn-clear-log') as HTMLButtonElement
const logList = document.getElementById('log-list') as HTMLUListElement
const btnChromeLogin = document.getElementById('btn-chrome-login') as HTMLButtonElement
const btnReset = document.getElementById('btn-reset') as HTMLButtonElement
const btnRediagnose = document.getElementById('btn-rediagnose') as HTMLButtonElement
const btnSaveProfile = document.getElementById('btn-save-profile') as HTMLButtonElement
const btnRemoveProfile = document.getElementById('btn-remove-profile') as HTMLButtonElement
const profileSelect = document.getElementById('profile-select') as HTMLSelectElement
const chkDownload = document.getElementById('chk-download') as HTMLInputElement
const saveProfileDialog = document.getElementById('save-profile-dialog') as HTMLDialogElement
const saveProfileForm = document.getElementById('save-profile-form') as HTMLFormElement
const saveProfileName = document.getElementById('save-profile-name') as HTMLInputElement
const saveProfileCancel = document.getElementById('save-profile-cancel') as HTMLButtonElement
const btnZoomOut = document.getElementById('btn-zoom-out') as HTMLButtonElement
const btnZoomIn = document.getElementById('btn-zoom-in') as HTMLButtonElement
const btnZoomReset = document.getElementById('btn-zoom-reset') as HTMLButtonElement
const zoomLevel = document.getElementById('zoom-level') as HTMLElement
const logSection = document.getElementById('log-section') as HTMLElement
const statusState = document.getElementById('status-state') as HTMLElement
const statusCount = document.getElementById('status-count') as HTMLElement
const statusProgress = document.getElementById('status-progress') as HTMLProgressElement
const statusPctLabel = document.getElementById('status-pct-label') as HTMLElement
const statusTime = document.getElementById('status-time') as HTMLElement
const errorAlert = document.getElementById('error-alert') as HTMLElement

let isPaused = false
let isRunningUi = false
let pageStatus: PageStatus = { isPhotosGoogle: false, isLoginBlocked: false }
let activeProfileId: string | null = null

const STATE_LABELS: Record<EngineState, string> = {
  idle: 'Inattivo',
  running: 'In corso',
  paused: 'In pausa',
  stopped: 'Interrotto',
  downloading: 'In download...',
  done: 'Completato',
  error: 'Errore'
}

function showError(msg: string): void {
  errorAlert.textContent = msg
  errorAlert.classList.remove('hidden')
}

function formatTime(ms: number): string {
  const sec = Math.floor(ms / 1000)
  const m = Math.floor(sec / 60)
  const s = sec % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

const SLIDER_PARAMS = [
  'clickDelay',
  'scrollFraction',
  'settleDelay',
  'passesPerStep',
  'stallPasses',
  'maxSelections'
] as const satisfies ReadonlyArray<keyof typeof PARAM_RANGES>

function syncSliderOutput(name: string): void {
  const input = form.elements.namedItem(name) as HTMLInputElement | null
  const output = document.getElementById(`${name}-value`)
  if (input && output) output.textContent = input.value
}

function initParamSliders(): void {
  for (const key of SLIDER_PARAMS) {
    const input = form.elements.namedItem(key) as HTMLInputElement | null
    if (!input) continue
    const range = PARAM_RANGES[key]
    input.min = String(range.min)
    input.max = String(range.max)
    input.step = '1'
    input.addEventListener('input', () => syncSliderOutput(key))
    syncSliderOutput(key)
  }
}

function applyParamTooltips(): void {
  for (const [key, tip] of Object.entries(PARAM_TOOLTIPS)) {
    const input = form.elements.namedItem(key)
    const label = input instanceof HTMLElement ? input.closest("label") : null
    if (label) label.title = tip
  }
}

function setRunControlsMode(active: boolean): void {
  isRunningUi = active
  actionsSection.classList.toggle('actions-idle', !active)
  actionsSection.classList.toggle('actions-active', active)
  btnPause.classList.toggle('hidden', !active)

  if (active) {
    btnStart.textContent = '⏹ Stop'
    btnStart.classList.add('btn-running')
    btnStart.disabled = false
    btnDelete.disabled = true
  } else {
    btnStart.textContent = '▶ Avvia'
    btnStart.classList.remove('btn-running')
    updatePageUi()
  }
}

function readParamsFromForm(): SelectionParams {
  const fd = new FormData(form)
  return {
    clickDelay: Number(fd.get('clickDelay')),
    scrollFraction: Number(fd.get('scrollFraction')),
    settleDelay: Number(fd.get('settleDelay')),
    passesPerStep: Number(fd.get('passesPerStep')),
    stallPasses: Number(fd.get('stallPasses')),
    skipLabelPrefix: String(fd.get('skipLabelPrefix') ?? ''),
    maxSelections: Number(fd.get('maxSelections')),
    downloadAfterSelection: chkDownload.checked
  }
}

function fillForm(params: SelectionParams): void {
  for (const [key, value] of Object.entries(params)) {
    if (key === 'downloadAfterSelection') {
      chkDownload.checked = Boolean(value)
      continue
    }
    const input = form.elements.namedItem(key) as HTMLInputElement | null
    if (!input) continue
    if (input.type === 'checkbox') {
      input.checked = Boolean(value)
    } else {
      input.value = String(value)
      syncSliderOutput(key)
    }
  }
}

function updateProfileDeleteButton(): void {
  btnRemoveProfile.classList.toggle('hidden', !profileSelect.value)
}

function canRunAutomation(): boolean {
  return pageStatus.isPhotosGoogle && !pageStatus.isLoginBlocked
}

function updatePageUi(): void {
  if (isRunningUi) return

  const canStart = canRunAutomation()
  btnStart.disabled = !canStart
  btnDelete.disabled = !canStart
}

function updateProgress(p: ProgressEvent): void {
  statusState.textContent = STATE_LABELS[p.state] ?? p.state
  statusCount.textContent = String(p.selected)
  const pct = Math.min(100, Math.max(0, p.estPct))
  statusProgress.value = pct
  statusPctLabel.textContent = `${Math.round(pct)}%`
  statusTime.textContent = formatTime(p.elapsedMs)
  if (p.warning) showError(p.warning)
  statusProgress.setAttribute(
    'aria-label',
    p.state === 'downloading' ? 'Avanzamento download' : 'Avanzamento selezione'
  )

  const running = p.state === 'running'
  const paused = p.state === 'paused'
  isPaused = paused
  setRunControlsMode(running || paused)
  btnPause.disabled = !running && !paused
  btnPause.textContent = paused ? '▶ Riprendi' : '⏸ Pausa'
}

function fillProfileSelect(store: ProfileStoreData): void {
  activeProfileId = store.activeProfileId
  profileSelect.innerHTML = '<option value="">Temporanea (non salvata)</option>'
  for (const profile of store.profiles) {
    const opt = document.createElement('option')
    opt.value = profile.id
    opt.textContent = profile.name
    profileSelect.appendChild(opt)
  }
  profileSelect.value = store.activeProfileId ?? ''
  updateProfileDeleteButton()
}

async function refreshProfiles(): Promise<void> {
  fillProfileSelect(await api.listProfiles())
}

function updateZoomLabel(percent: number): void {
  zoomLevel.textContent = `${percent}%`
}

async function refreshZoomLabel(): Promise<void> {
  const state = await api.getZoom()
  updateZoomLabel(state.percent)
}

function updateDevModeUi(enabled: boolean): void {
  logSection.classList.toggle('hidden', !enabled)
}

async function refreshDevMode(): Promise<void> {
  const state = await api.getDevMode()
  updateDevModeUi(state.enabled)
}

function bindZoomControls(): void {
  btnZoomOut.addEventListener('click', () => {
    void api.zoomStep(-1).then((state) => updateZoomLabel(state.percent))
  })
  btnZoomIn.addEventListener('click', () => {
    void api.zoomStep(1).then((state) => updateZoomLabel(state.percent))
  })
  btnZoomReset.addEventListener('click', () => {
    void api.zoomReset().then((state) => updateZoomLabel(state.percent))
  })
}

function appendLog(event: LogEvent): void {
  const li = document.createElement('li')
  li.className = `level-${event.level}`
  const stamp = new Date(event.ts).toLocaleTimeString('it-IT', { hour12: false })
  li.textContent = `[${stamp}] [${event.level}] ${event.msg}`
  logList.appendChild(li)
  logList.scrollTop = logList.scrollHeight
}

function bindButtons(): void {
  btnClearLog.addEventListener('click', () => {
    logList.innerHTML = ''
    void api.clearLog()
  })

  btnChromeLogin.addEventListener('click', () => {
    void (async () => {
      btnChromeLogin.disabled = true
      errorAlert.classList.add('hidden')
      try {
        const result = await api.chromeLogin()
        if (!result.ok) {
          showError(result.message ?? 'Login non riuscito.')
        } else {
          pageStatus = (await api.getPageStatus()) ?? pageStatus
          updatePageUi()
        }
      } catch (err) {
        showError(err instanceof Error ? err.message : 'Errore login Chrome.')
      } finally {
        btnChromeLogin.disabled = false
      }
    })()
  })

  btnStart.addEventListener('click', () => {
    if (isRunningUi) {
      api.stop()
      return
    }

    void (async () => {
      errorAlert.classList.add('hidden')
      btnRediagnose.classList.add('hidden')
      try {
        const params = readParamsFromForm()
        await api.saveSettings(params)
        setRunControlsMode(true)
        await api.start(params)
        statusState.textContent = 'In corso'
      } catch (err) {
        setRunControlsMode(false)
        showError(err instanceof Error ? err.message : 'Errore avvio selezione.')
      }
    })()
  })

  btnDelete.addEventListener('click', () => {
    if (isRunningUi) return
    const confirmed = window.confirm(
      chkDownload.checked
        ? 'Sei sicuro di voler selezionare tutte le foto visibili, scaricarle e poi spostarle nel cestino?'
        : 'Sei sicuro di voler selezionare tutte le foto visibili e spostarle nel cestino?'
    )
    if (!confirmed) return

    void (async () => {
      errorAlert.classList.add('hidden')
      btnRediagnose.classList.add('hidden')
      try {
        const params = readParamsFromForm()
        await api.saveSettings(params)
        setRunControlsMode(true)
        await api.deleteSelected(params)
        statusState.textContent = 'In corso'
      } catch (err) {
        setRunControlsMode(false)
        showError(err instanceof Error ? err.message : 'Errore avvio eliminazione.')
      }
    })()
  })

  btnPause.addEventListener('click', () => {
    if (isPaused) api.resume()
    else api.pause()
  })

  btnReset.addEventListener('click', () => {
    void (async () => {
      const defaults = await api.resetSettings()
      fillForm(defaults)
    })()
  })

  btnRediagnose.addEventListener('click', () => {
    void (async () => {
      const diag = await api.rediagnose()
      showError(
        `Diagnostica: scroll=${diag.scrollContainers}, main=${diag.mainElements}, checkbox=${diag.checkboxes}`
      )
    })()
  })

  profileSelect.addEventListener('change', () => {
    updateProfileDeleteButton()
    void (async () => {
      const profileId = profileSelect.value || null
      if (profileId === activeProfileId) return
      profileSelect.disabled = true
      try {
        const store = await api.switchProfile(profileId)
        fillProfileSelect(store)
        pageStatus = (await api.getPageStatus()) ?? pageStatus
        updatePageUi()
      } catch (err) {
        showError(err instanceof Error ? err.message : 'Errore cambio profilo.')
        profileSelect.value = activeProfileId ?? ''
      } finally {
        profileSelect.disabled = false
      }
    })()
  })

  btnSaveProfile.addEventListener('click', () => {
    saveProfileName.value = ''
    saveProfileDialog.showModal()
    saveProfileName.focus()
  })

  saveProfileCancel.addEventListener('click', () => {
    saveProfileDialog.close()
  })

  saveProfileForm.addEventListener('submit', (event) => {
    event.preventDefault()
    const name = saveProfileName.value.trim()
    if (!name) {
      saveProfileName.focus()
      return
    }
    saveProfileDialog.close()

    void (async () => {
      btnSaveProfile.disabled = true
      try {
        const result = await api.saveProfile(name)
        if (!result.ok) {
          showError(result.message ?? 'Salvataggio non riuscito.')
          return
        }
        await refreshProfiles()
      } catch (err) {
        showError(err instanceof Error ? err.message : 'Errore salvataggio profilo.')
      } finally {
        btnSaveProfile.disabled = false
      }
    })()
  })

  btnRemoveProfile.addEventListener('click', () => {
    const profileId = profileSelect.value
    if (!profileId) return
    const label = profileSelect.selectedOptions[0]?.textContent ?? 'questa sessione'
    const confirmed = window.confirm(`Eliminare la sessione salvata «${label}»?`)
    if (!confirmed) return

    void (async () => {
      btnRemoveProfile.disabled = true
      try {
        const result = await api.deleteProfile(profileId)
        if (!result.ok) {
          showError(result.message ?? 'Eliminazione non riuscita.')
          return
        }
        await refreshProfiles()
        pageStatus = (await api.getPageStatus()) ?? pageStatus
        updatePageUi()
      } catch (err) {
        showError(err instanceof Error ? err.message : 'Errore eliminazione sessione.')
      } finally {
        btnRemoveProfile.disabled = false
      }
    })()
  })
}

export async function startPanel(): Promise<void> {
  initParamSliders()
  applyParamTooltips()
  bindZoomControls()
  bindButtons()
  setRunControlsMode(false)

  api.onProgress(updateProgress)
  api.onLog(appendLog)
  api.onLoginStatus((message) => {
    statusState.textContent = message
  })
  api.onPageStatus((status) => {
    pageStatus = status
    updatePageUi()
  })
  api.onZoomChanged(({ factor, percent }) => {
    updateZoomLabel(percent ?? Math.round(factor * 100))
  })
  api.onDevModeChanged(({ enabled }) => {
    updateDevModeUi(enabled)
  })
  api.onDone((event) => {
    statusState.textContent = 'Completato'
    statusCount.textContent = String(event.total)
    statusProgress.value = 100
    statusPctLabel.textContent = '100%'
    if (event.leftovers > 0) {
      showError(`Attenzione: ${event.leftovers} checkbox non selezionate.`)
    }
    setRunControlsMode(false)
  })
  api.onError((event) => {
    showError(
      `${event.message} (scroll: ${event.diagnostics.scrollContainers}, main: ${event.diagnostics.mainElements}, checkbox: ${event.diagnostics.checkboxes})`
    )
    btnRediagnose.classList.remove('hidden')
    statusState.textContent = 'Errore'
    setRunControlsMode(false)
  })

  try {
    const settings = await api.getSettings()
    fillForm(settings)
    await refreshProfiles()
    await refreshZoomLabel()
    await refreshDevMode()
    for (const entry of await api.logHistory()) appendLog(entry)
    pageStatus = await api.getPageStatus()
    updatePageUi()
  } catch (err) {
    showError(err instanceof Error ? err.message : 'Errore caricamento impostazioni.')
  }

  form.addEventListener('change', () => {
    void api.saveSettings(readParamsFromForm())
  })

  chkDownload.addEventListener('change', () => {
    void api.saveSettings(readParamsFromForm())
  })

  const tutorial = attachTutorial((dismissForever) => {
    if (dismissForever) void api.setTutorialCompleted(true)
  })

  try {
    const { completed } = await api.getTutorialCompleted()
    if (!completed) {
      requestAnimationFrame(() => tutorial.start())
    }
  } catch {
    requestAnimationFrame(() => tutorial.start())
  }
}
