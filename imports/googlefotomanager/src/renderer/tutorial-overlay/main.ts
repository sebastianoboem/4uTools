import type { TutorialRenderPayload } from '../../shared/tutorial-types'

declare global {
  interface Window {
    tutorialOverlayApi?: {
      action: (type: 'skip' | 'next', dismissForever: boolean) => void
      onRender: (cb: (payload: TutorialRenderPayload) => void) => () => void
    }
  }
}

const spotlight = document.getElementById('tutorial-spotlight') as HTMLElement
const card = document.getElementById('tutorial-card') as HTMLElement
const textEl = document.getElementById('tutorial-text') as HTMLElement
const stepLabel = document.getElementById('tutorial-step') as HTMLElement
const dismissCheckbox = document.getElementById('tutorial-dismiss') as HTMLInputElement
const btnSkip = document.getElementById('tutorial-skip') as HTMLButtonElement
const btnNext = document.getElementById('tutorial-next') as HTMLButtonElement

function render(payload: TutorialRenderPayload): void {
  if (payload.mode === 'intro') dismissCheckbox.checked = false

  textEl.innerHTML = payload.html
  stepLabel.textContent = payload.stepLabel
  btnSkip.textContent = payload.skipLabel
  btnNext.textContent = payload.nextLabel

  if (payload.spotlight) {
    spotlight.classList.remove('hidden')
    spotlight.style.top = `${payload.spotlight.y}px`
    spotlight.style.left = `${payload.spotlight.x}px`
    spotlight.style.width = `${payload.spotlight.width}px`
    spotlight.style.height = `${payload.spotlight.height}px`
  } else {
    spotlight.classList.add('hidden')
  }

  card.style.maxWidth = `${payload.card.maxWidth}px`
  if (payload.card.centered) {
    card.classList.add('is-centered')
    card.style.top = `${payload.card.top}px`
    card.style.left = `${payload.card.left}px`
  } else {
    card.classList.remove('is-centered')
    card.style.top = `${payload.card.top}px`
    card.style.left = `${payload.card.left}px`
  }
}

btnSkip.addEventListener('click', () => {
  window.tutorialOverlayApi?.action('skip', dismissCheckbox.checked)
})

btnNext.addEventListener('click', () => {
  window.tutorialOverlayApi?.action('next', dismissCheckbox.checked)
})

window.tutorialOverlayApi?.onRender(render)
