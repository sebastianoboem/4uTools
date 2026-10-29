import { describe, expect, it } from 'vitest'
import { findMainElement, findScrollContainer } from '../../src/engine/selectors'

describe('findScrollContainer', () => {
  it('finds c-wiz with overflow auto', () => {
    document.body.innerHTML = `
      <c-wiz style="overflow-y: auto; height: 200px;">
        <div role="main"><div role="checkbox"></div></div>
      </c-wiz>
    `
    const wiz = document.querySelector('c-wiz') as HTMLElement
    Object.defineProperty(wiz, 'scrollHeight', { configurable: true, value: 800 })
    Object.defineProperty(wiz, 'clientHeight', { configurable: true, value: 200 })

    expect(findScrollContainer()).toBe(wiz)
  })

  it('finds scrollable ancestor of role=main when c-wiz overflow is hidden', () => {
    document.body.innerHTML = `
      <c-wiz style="overflow-y: hidden; height: 400px;">
        <div role="main" style="overflow-y: auto; height: 400px;">
          <div role="checkbox"></div>
        </div>
      </c-wiz>
    `
    const main = findMainElement()!
    Object.defineProperty(main, 'scrollHeight', { configurable: true, value: 1200 })
    Object.defineProperty(main, 'clientHeight', { configurable: true, value: 400 })

    expect(findScrollContainer()).toBe(main)
  })

  it('picks c-wiz containing main when it has scrollable content', () => {
    document.body.innerHTML = `
      <c-wiz style="overflow-y: hidden; height: 300px;">
        <div role="main"><div role="checkbox"></div></div>
      </c-wiz>
    `
    const wiz = document.querySelector('c-wiz') as HTMLElement
    Object.defineProperty(wiz, 'scrollHeight', { configurable: true, value: 900 })
    Object.defineProperty(wiz, 'clientHeight', { configurable: true, value: 300 })

    expect(findScrollContainer()).toBe(wiz)
  })
})
