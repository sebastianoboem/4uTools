import { describe, expect, it } from 'vitest'
import {
  clampParams,
  DEFAULT_SELECTION_PARAMS,
  isDefaultInRange,
  PARAM_RANGES
} from '../../src/tabs/gphotos/engine/settings-defaults'

describe('settings defaults', () => {
  it('defaults are within defined ranges', () => {
    expect(isDefaultInRange()).toBe(true)
  })

  it('each default value respects min/max', () => {
    const p = DEFAULT_SELECTION_PARAMS
    expect(p.clickDelay).toBeGreaterThanOrEqual(PARAM_RANGES.clickDelay.min)
    expect(p.clickDelay).toBeLessThanOrEqual(PARAM_RANGES.clickDelay.max)
    expect(p.scrollFraction).toBeGreaterThanOrEqual(PARAM_RANGES.scrollFraction.min)
    expect(p.scrollFraction).toBeLessThanOrEqual(PARAM_RANGES.scrollFraction.max)
    expect(p.settleDelay).toBeGreaterThanOrEqual(PARAM_RANGES.settleDelay.min)
    expect(p.settleDelay).toBeLessThanOrEqual(PARAM_RANGES.settleDelay.max)
    expect(p.passesPerStep).toBeGreaterThanOrEqual(PARAM_RANGES.passesPerStep.min)
    expect(p.passesPerStep).toBeLessThanOrEqual(PARAM_RANGES.passesPerStep.max)
    expect(p.stallPasses).toBeGreaterThanOrEqual(PARAM_RANGES.stallPasses.min)
    expect(p.stallPasses).toBeLessThanOrEqual(PARAM_RANGES.stallPasses.max)
  })

  it('maps a saved scroll step above 80 back to the default', () => {
    expect(clampParams({ ...DEFAULT_SELECTION_PARAMS, scrollFraction: 150 }).scrollFraction).toBe(70)
    expect(clampParams({ ...DEFAULT_SELECTION_PARAMS, scrollFraction: 80 }).scrollFraction).toBe(80)
    expect(clampParams({ ...DEFAULT_SELECTION_PARAMS, scrollFraction: 40 }).scrollFraction).toBe(40)
  })
})
