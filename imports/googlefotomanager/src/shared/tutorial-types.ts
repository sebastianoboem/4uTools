export interface TutorialRect {
  x: number
  y: number
  width: number
  height: number
}

export interface TutorialCardLayout {
  top: number
  left: number
  maxWidth: number
  centered?: boolean
}

export interface TutorialRenderPayload {
  mode: 'intro' | 'step'
  html: string
  stepLabel: string
  skipLabel: string
  nextLabel: string
  spotlight: TutorialRect | null
  card: TutorialCardLayout
}

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
