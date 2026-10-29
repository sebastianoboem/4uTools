import type {
  EngineState,
  ErrorEvent,
  PageStatus,
  ProfileStoreData,
  ProgressEvent,
  SelectionParams
} from '../../shared/types'
import { PARAM_RANGES, PARAM_TOOLTIPS } from '../../shared/settings-defaults'
import { createTutorial } from './tutorial'

declare global {
  interface Window {
    panelApi?: {
      getSettings: () => Promise<SelectionParams>
      saveSettings: (params: SelectionParams) => Promise<SelectionParams>
      resetSettings: () => Promise<SelectionParams>
      start: (params: SelectionParams) => Promise<void>
      deleteSelected: (params: SelectionParams) => Promise<void>
      pause: () => void
      resume: () => void
      stop: () => void
      openLog: () => void
      chromeLogin: () => Promise<{ ok: boolean; message?: string; cookieCount?: number }>
      rediagnose: () => Promise<unknown>
      getPageStatus: () => Promise<PageStatus>
      listProfiles: () => Promise<ProfileStoreData>
      switchProfile: (profileId: string | null) => Promise<ProfileStoreData>
      saveProfile: (name: string) => Promise<{ ok: boolean; message?: string }>
      deleteProfile: (profileId: string) => Promise<{ ok: boolean; message?: string }>
      getZoom: () => Promise<{ factor: number; percent: number }>
      getDevMode: () => Promise<{ enabled: boolean }>
      getAppVersion: () => Promise<string>
      getTutorialCompleted: () => Promise<{ completed: boolean }>
      setTutorialCompleted: (completed: boolean) => Promise<{ completed: boolean }>
      tutorialShow: () => void
      tutorialHide: () => void
      tutorialRender: (request: import('../../shared/tutorial-types').TutorialRenderRequest) => Promise<void>
      onTutorialAction: (cb: (action: import('../../shared/tutorial-types').TutorialActionPayload) => void) => () => void
      onTutorialReposition: (cb: () => void) => () => void
      zoomStep: (direction: -1 | 1) => Promise<{ factor: number; percent: number }>
      zoomReset: () => Promise<{ factor: number; percent: number }>
      onZoomChanged: (cb: (state: { factor: number; percent?: number }) => void) => () => void
      onDevModeChanged: (cb: (state: { enabled: boolean }) => void) => () => void
      onProgress: (cb: (event: ProgressEvent) => void) => () => void
      onDone: (cb: (event: { total: number; leftovers: number }) => void) => () => void
      onError: (cb: (event: ErrorEvent) => void) => () => void
      onPageStatus: (cb: (status: PageStatus) => void) => () => void
    }
  }
}

const form = document.getElementById('params-form') as HTMLFormElement
const actionsSection = document.getElementById('actions-section') as HTMLElement
const btnStart = document.getElementById('btn-start') as HTMLButtonElement
const btnDelete = document.getElementById('btn-delete') as HTMLButtonElement
const btnPause = document.getElementById('btn-pause') as HTMLButtonElement
const btnLog = document.getElementById('btn-log') as HTMLButtonElement
const btnChromeLogin = document.getElementById('btn-chrome-login') as HTMLButtonElement
const appVersionEl = document.getElementById('app-version') as HTMLParagraphElement
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
    const label = input?.closest('label')
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
  if (!window.panelApi?.listProfiles) return
  fillProfileSelect(await window.panelApi.listProfiles())
}

function updateZoomLabel(percent: number): void {
  zoomLevel.textContent = `${percent}%`
}

async function refreshZoomLabel(): Promise<void> {
  if (!window.panelApi?.getZoom) return
  const state = await window.panelApi.getZoom()
  updateZoomLabel(state.percent)
}

function updateDevModeUi(enabled: boolean): void {
  logSection.classList.toggle('hidden', !enabled)
}

async function refreshDevMode(): Promise<void> {
  if (!window.panelApi?.getDevMode) return
  const state = await window.panelApi.getDevMode()
  updateDevModeUi(state.enabled)
}

function bindZoomControls(): void {
  btnZoomOut.addEventListener('click', () => {
    void window.panelApi?.zoomStep(-1).then((state) => updateZoomLabel(state.percent))
  })
  btnZoomIn.addEventListener('click', () => {
    void window.panelApi?.zoomStep(1).then((state) => updateZoomLabel(state.percent))
  })
  btnZoomReset.addEventListener('click', () => {
    void window.panelApi?.zoomReset().then((state) => updateZoomLabel(state.percent))
  })
}

function bindButtons(): void {
  btnLog.addEventListener('click', () => {
    if (!window.panelApi?.openLog) {
      showError('Pannello non collegato al processo principale (preload). Riavvia l\'app.')
      return
    }
    window.panelApi.openLog()
  })

  btnChromeLogin.addEventListener('click', () => {
    void (async () => {
      if (!window.panelApi?.chromeLogin) {
        showError('Pannello non collegato al processo principale (preload). Riavvia l\'app.')
        return
      }
      btnChromeLogin.disabled = true
      errorAlert.classList.add('hidden')
      try {
        const result = await window.panelApi.chromeLogin()
        if (!result.ok) {
          showError(result.message ?? 'Login non riuscito.')
        } else {
          pageStatus = (await window.panelApi.getPageStatus()) ?? pageStatus
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
      window.panelApi?.stop()
      return
    }

    void (async () => {
      if (!window.panelApi?.start) {
        showError('Preload non disponibile.')
        return
      }
      errorAlert.classList.add('hidden')
      btnRediagnose.classList.add('hidden')
      try {
        const params = readParamsFromForm()
        await window.panelApi.saveSettings(params)
        setRunControlsMode(true)
        await window.panelApi.start(params)
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
      if (!window.panelApi?.deleteSelected) {
        showError('Preload non disponibile.')
        return
      }
      errorAlert.classList.add('hidden')
      btnRediagnose.classList.add('hidden')
      try {
        const params = readParamsFromForm()
        await window.panelApi.saveSettings(params)
        setRunControlsMode(true)
        await window.panelApi.deleteSelected(params)
        statusState.textContent = 'In corso'
      } catch (err) {
        setRunControlsMode(false)
        showError(err instanceof Error ? err.message : 'Errore avvio eliminazione.')
      }
    })()
  })

  btnPause.addEventListener('click', () => {
    if (isPaused) window.panelApi?.resume()
    else window.panelApi?.pause()
  })

  btnReset.addEventListener('click', () => {
    void (async () => {
      if (!window.panelApi?.resetSettings) return
      const defaults = await window.panelApi.resetSettings()
      fillForm(defaults)
    })()
  })

  btnRediagnose.addEventListener('click', () => {
    void (async () => {
      if (!window.panelApi?.rediagnose) return
      const diag = (await window.panelApi.rediagnose()) as {
        scrollContainers: number
        mainElements: number
        checkboxes: number
      }
      showError(
        `Diagnostica: scroll=${diag.scrollContainers}, main=${diag.mainElements}, checkbox=${diag.checkboxes}`
      )
    })()
  })

  profileSelect.addEventListener('change', () => {
    updateProfileDeleteButton()
    void (async () => {
      if (!window.panelApi?.switchProfile) return
      const profileId = profileSelect.value || null
      if (profileId === activeProfileId) return
      profileSelect.disabled = true
      try {
        const store = await window.panelApi.switchProfile(profileId)
        fillProfileSelect(store)
        pageStatus = (await window.panelApi.getPageStatus()) ?? pageStatus
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
      if (!window.panelApi?.saveProfile) return
      btnSaveProfile.disabled = true
      try {
        const result = await window.panelApi.saveProfile(name)
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
      if (!window.panelApi?.deleteProfile) return
      btnRemoveProfile.disabled = true
      try {
        const result = await window.panelApi.deleteProfile(profileId)
        if (!result.ok) {
          showError(result.message ?? 'Eliminazione non riuscita.')
          return
        }
        await refreshProfiles()
        pageStatus = (await window.panelApi.getPageStatus()) ?? pageStatus
        updatePageUi()
      } catch (err) {
        showError(err instanceof Error ? err.message : 'Errore eliminazione sessione.')
      } finally {
        btnRemoveProfile.disabled = false
      }
    })()
  })
}

async function init(): Promise<void> {
  initParamSliders()
  applyParamTooltips()
  bindZoomControls()
  bindButtons()
  setRunControlsMode(false)

  if (!window.panelApi) {
    showError('Bridge IPC non caricato. Chiudi e rilancia con npm run dev.')
    return
  }

  window.panelApi.onProgress(updateProgress)
  window.panelApi.onPageStatus((status) => {
    pageStatus = status
    updatePageUi()
  })
  window.panelApi.onZoomChanged(({ factor, percent }) => {
    updateZoomLabel(percent ?? Math.round(factor * 100))
  })
  window.panelApi.onDevModeChanged(({ enabled }) => {
    updateDevModeUi(enabled)
  })
  window.panelApi.onDone((event) => {
    statusState.textContent = 'Completato'
    statusCount.textContent = String(event.total)
    statusProgress.value = 100
    statusPctLabel.textContent = '100%'
    if (event.leftovers > 0) {
      showError(`Attenzione: ${event.leftovers} checkbox non selezionate.`)
    }
    setRunControlsMode(false)
  })
  window.panelApi.onError((event) => {
    showError(
      `${event.message} (scroll: ${event.diagnostics.scrollContainers}, main: ${event.diagnostics.mainElements}, checkbox: ${event.diagnostics.checkboxes})`
    )
    btnRediagnose.classList.remove('hidden')
    statusState.textContent = 'Errore'
    setRunControlsMode(false)
  })

  try {
    const settings = await window.panelApi.getSettings()
    const version = await window.panelApi.getAppVersion()
    appVersionEl.textContent = `v${version}`
    fillForm(settings)
    await refreshProfiles()
    await refreshZoomLabel()
    await refreshDevMode()
    pageStatus = await window.panelApi.getPageStatus()
    updatePageUi()
  } catch (err) {
    showError(err instanceof Error ? err.message : 'Errore caricamento impostazioni.')
  }

  form.addEventListener('change', () => {
    void window.panelApi?.saveSettings(readParamsFromForm())
  })

  chkDownload.addEventListener('change', () => {
    void window.panelApi?.saveSettings(readParamsFromForm())
  })

  const tutorial = createTutorial(
    {
      show: () => window.panelApi!.tutorialShow(),
      hide: () => window.panelApi!.tutorialHide(),
      render: (request) => window.panelApi!.tutorialRender(request),
      onAction: (cb) => window.panelApi!.onTutorialAction(cb),
      onReposition: (cb) => window.panelApi!.onTutorialReposition(cb)
    },
    (dismissForever) => {
      if (dismissForever) void window.panelApi?.setTutorialCompleted(true)
    }
  )

  try {
    const { completed } = await window.panelApi.getTutorialCompleted()
    if (!completed) {
      requestAnimationFrame(() => tutorial.start())
    }
  } catch {
    requestAnimationFrame(() => tutorial.start())
  }
}

void init()
