import { describe, expect, it } from 'vitest'
import { findClearSelectionButton } from '../../src/engine/selectors'

describe('findClearSelectionButton', () => {
  it('finds button with Italian aria-label', () => {
    document.body.innerHTML =
      '<button aria-label="Cancella selezione">X</button><div role="main"></div>'
    expect(findClearSelectionButton(document)?.getAttribute('aria-label')).toBe(
      'Cancella selezione'
    )
  })
})
