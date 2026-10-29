import type { TutorialActionPayload, TutorialRenderRequest } from '../../shared/tutorial-types'
import { TUTORIAL_INTRO_HTML, TUTORIAL_STEPS } from '../../shared/tutorial-steps'

export { TUTORIAL_STEPS, TUTORIAL_INTRO_HTML }

export interface TutorialController {
  start: () => void
}

export function createTutorial(
  api: {
    show: () => void
    hide: () => void
    render: (request: TutorialRenderRequest) => Promise<void>
    onAction: (cb: (action: TutorialActionPayload) => void) => () => void
    onReposition: (cb: () => void) => () => void
  },
  onComplete: (dismissForever: boolean) => void
): TutorialController {
  let stepIndex = 0
  let active = false
  let introMode = false
  let lastRequest: TutorialRenderRequest | null = null

  function finish(dismissForever: boolean): void {
    active = false
    introMode = false
    lastRequest = null
    api.hide()
    onComplete(dismissForever)
  }

  function showIntro(): void {
    introMode = true
    lastRequest = {
      mode: 'intro',
      html: TUTORIAL_INTRO_HTML,
      stepLabel: 'Tutorial',
      skipLabel: 'Salta tutorial',
      nextLabel: 'Inizia'
    }
    void api.render(lastRequest)
  }

  function showStep(index: number): void {
    introMode = false
    const step = TUTORIAL_STEPS[index]
    lastRequest = {
      mode: 'step',
      html: step.html,
      stepLabel: `${index + 1} / ${TUTORIAL_STEPS.length}`,
      skipLabel: 'Salta',
      nextLabel: index === TUTORIAL_STEPS.length - 1 ? 'Fine' : 'Avanti',
      targetSelector: step.target
    }
    void api.render(lastRequest)
  }

  function start(): void {
    if (active) return
    active = true
    stepIndex = 0
    api.show()
    showIntro()
  }

  api.onAction((action) => {
    if (!active) return

    if (action.type === 'skip') {
      finish(action.dismissForever)
      return
    }

    if (introMode) {
      showStep(stepIndex)
      return
    }

    if (stepIndex >= TUTORIAL_STEPS.length - 1) {
      finish(action.dismissForever)
      return
    }

    stepIndex++
    showStep(stepIndex)
  })

  api.onReposition(() => {
    if (!active || !lastRequest) return
    void api.render(lastRequest)
  })

  return { start }
}
