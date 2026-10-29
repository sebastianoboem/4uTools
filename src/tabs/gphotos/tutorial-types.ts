export type TutorialActionType = 'skip' | 'next'

export interface TutorialActionPayload {
  type: TutorialActionType
  dismissForever: boolean
}

export interface TutorialRenderRequest {
  mode: 'intro' | 'step'
  html: string
  stepLabel: string
  skipLabel: string
  nextLabel: string
  targetSelector?: string
}
