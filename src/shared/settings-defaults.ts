import type { SelectionParams } from './types'

export const DEFAULT_SELECTION_PARAMS: SelectionParams = {
  clickDelay: 80,
  scrollFraction: 70,
  settleDelay: 1200,
  passesPerStep: 2,
  stallPasses: 6,
  skipLabelPrefix: 'Salva tutte le foto',
  maxSelections: 0,
  downloadAfterSelection: false
}

export const PARAM_TOOLTIPS: Record<keyof SelectionParams, string> = {
  clickDelay:
    'Pausa in millisecondi tra un click e il successivo sulle checkbox visibili, per dare tempo alla UI di aggiornarsi.',
  scrollFraction:
    'Quanto scorrere a ogni passo, in percentuale del tratto che tiene ancora visibile l’ultima foto già montata. Valori più alti avanzano di più, senza uscire da quella fascia.',
  settleDelay:
    'Tempo massimo in millisecondi per il caricamento delle anteprime dopo lo scroll. Il motore prosegue appena le anteprime nuove sono caricate.',
  passesPerStep:
    'Quante volte ripetere la selezione sulle checkbox visibili prima e dopo ogni scroll (anti-skip).',
  stallPasses:
    'Quante volte consecutive l’altezza dello scroll non cambia prima di considerare la libreria esaurita.',
  skipLabelPrefix:
    'Le checkbox il cui aria-label inizia con questo testo non vengono cliccate (es. «Salva tutte le foto»).',
  maxSelections:
    'Limite massimo di click di selezione eseguiti dal motore (0 = nessun limite). Il contatore «Selezionate» segue il totale mostrato da Google Foto.',
  downloadAfterSelection:
    'Con Avvia: al termine della selezione apre il menu e scarica. Con Elimina: scarica e poi sposta nel cestino.'
}

export const PARAM_RANGES = {
  clickDelay: { min: 0, max: 500 },
  scrollFraction: { min: 20, max: 80 },
  settleDelay: { min: 200, max: 5000 },
  passesPerStep: { min: 1, max: 3 },
  stallPasses: { min: 3, max: 15 },
  maxSelections: { min: 0, max: 1_000_000 }
} as const

export function clampParams(params: SelectionParams): SelectionParams {
  return {
    clickDelay: clamp(params.clickDelay, PARAM_RANGES.clickDelay.min, PARAM_RANGES.clickDelay.max),
    scrollFraction: normalizeScrollFraction(params.scrollFraction),
    settleDelay: clamp(params.settleDelay, PARAM_RANGES.settleDelay.min, PARAM_RANGES.settleDelay.max),
    passesPerStep: clamp(params.passesPerStep, PARAM_RANGES.passesPerStep.min, PARAM_RANGES.passesPerStep.max),
    stallPasses: clamp(params.stallPasses, PARAM_RANGES.stallPasses.min, PARAM_RANGES.stallPasses.max),
    skipLabelPrefix: params.skipLabelPrefix.trim() || DEFAULT_SELECTION_PARAMS.skipLabelPrefix,
    maxSelections: clamp(params.maxSelections, PARAM_RANGES.maxSelections.min, PARAM_RANGES.maxSelections.max),
    downloadAfterSelection: Boolean(params.downloadAfterSelection)
  }
}

function normalizeScrollFraction(value: number): number {
  if (value > PARAM_RANGES.scrollFraction.max) return DEFAULT_SELECTION_PARAMS.scrollFraction
  return clamp(value, PARAM_RANGES.scrollFraction.min, PARAM_RANGES.scrollFraction.max)
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function isDefaultInRange(): boolean {
  const p = DEFAULT_SELECTION_PARAMS
  return (
    p.clickDelay >= PARAM_RANGES.clickDelay.min &&
    p.clickDelay <= PARAM_RANGES.clickDelay.max &&
    p.scrollFraction >= PARAM_RANGES.scrollFraction.min &&
    p.scrollFraction <= PARAM_RANGES.scrollFraction.max &&
    p.settleDelay >= PARAM_RANGES.settleDelay.min &&
    p.settleDelay <= PARAM_RANGES.settleDelay.max &&
    p.passesPerStep >= PARAM_RANGES.passesPerStep.min &&
    p.passesPerStep <= PARAM_RANGES.passesPerStep.max &&
    p.stallPasses >= PARAM_RANGES.stallPasses.min &&
    p.stallPasses <= PARAM_RANGES.stallPasses.max &&
    p.maxSelections >= PARAM_RANGES.maxSelections.min &&
    p.maxSelections <= PARAM_RANGES.maxSelections.max
  )
}
