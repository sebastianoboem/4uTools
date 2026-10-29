import { describe, expect, it } from 'vitest'
import {
  parseLocalizedInteger,
  parseSelectionCountText,
  readGooglePhotosSelectionCount
} from '../../src/engine/selectors'

describe('selection count from Google Foto UI', () => {
  it('parses Italian banner text', () => {
    expect(parseSelectionCountText('1389 elementi selezionati')).toBe(1389)
    expect(parseSelectionCountText('1 elemento selezionato')).toBe(1)
    expect(parseSelectionCountText('1.389 elementi selezionati')).toBe(1389)
  })

  it('parses English banner text', () => {
    expect(parseSelectionCountText('42 items selected')).toBe(42)
    expect(parseSelectionCountText('1 item selected')).toBe(1)
  })

  it('parseLocalizedInteger handles thousand separators', () => {
    expect(parseLocalizedInteger('1.389')).toBe(1389)
    expect(parseLocalizedInteger('12,345')).toBe(12345)
    expect(parseLocalizedInteger('380')).toBe(380)
  })

  it('reads count from toolbar in document', () => {
    document.body.innerHTML = `
      <div role="toolbar">1389 elementi selezionati</div>
      <div role="main"></div>
    `
    expect(readGooglePhotosSelectionCount(document)).toBe(1389)
  })
})
