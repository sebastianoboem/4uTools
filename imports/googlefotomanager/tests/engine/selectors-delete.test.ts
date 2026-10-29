import { describe, expect, it } from 'vitest'
import { findDeleteConfirmButton, findDeleteMenuItem } from '../../src/engine/selectors'

describe('delete selectors', () => {
  it('finds delete action in open submenu', () => {
    document.body.innerHTML = `
      <div role="menu">
        <div role="menuitem" aria-label="Sposta nel cestino">Sposta nel cestino</div>
      </div>
    `
    const item = findDeleteMenuItem()
    expect(item?.getAttribute('aria-label')).toBe('Sposta nel cestino')
  })

  it('finds confirm button only inside dialog', () => {
    document.body.innerHTML = `
      <button>Sposta nel cestino</button>
      <div role="dialog">
        <button>Sposta nel cestino</button>
      </div>
    `
    for (const btn of document.querySelectorAll('button')) {
      btn.getBoundingClientRect = () =>
        ({ width: 80, height: 32, x: 0, y: 0, top: 0, left: 0, right: 80, bottom: 32, toJSON: () => ({}) }) as DOMRect
    }
    const confirm = findDeleteConfirmButton()
    expect(confirm?.parentElement?.getAttribute('role')).toBe('dialog')
  })

  it('returns null for confirm when only submenu is open', () => {
    document.body.innerHTML = `
      <div role="menu">
        <div role="menuitem">Sposta nel cestino</div>
      </div>
    `
    expect(findDeleteConfirmButton()).toBeNull()
    expect(findDeleteMenuItem()?.textContent).toContain('Sposta nel cestino')
  })

  it('finds OK button in trash confirmation dialog', () => {
    document.body.innerHTML = `
      <div role="dialog">
        <button>Annulla</button>
        <button>OK</button>
      </div>
    `
    for (const btn of document.querySelectorAll('button')) {
      btn.getBoundingClientRect = () =>
        ({ width: 80, height: 32, x: 0, y: 0, top: 0, left: 0, right: 80, bottom: 32, toJSON: () => ({}) }) as DOMRect
    }
    expect(findDeleteConfirmButton()?.textContent).toBe('OK')
  })
})
