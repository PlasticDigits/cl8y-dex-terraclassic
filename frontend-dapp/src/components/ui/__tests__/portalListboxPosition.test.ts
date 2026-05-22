import { describe, expect, it } from 'vitest'
import { computePortalListboxStyle } from '../portalListboxPosition'

function rect(left: number, top: number, width: number, height: number): DOMRect {
  return {
    left,
    top,
    right: left + width,
    bottom: top + height,
    width,
    height,
    x: left,
    y: top,
    toJSON: () => ({}),
  } as DOMRect
}

describe('computePortalListboxStyle', () => {
  it('opens below the anchor when space allows', () => {
    const style = computePortalListboxStyle({
      anchor: rect(100, 200, 320, 48),
      viewport: { width: 1440, height: 900 },
      preferredMaxHeight: 280,
      gap: 8,
    })
    expect(style.position).toBe('fixed')
    expect(style.top).toBe(256)
    expect(style.left).toBe(100)
    expect(style.width).toBe(320)
    expect(style.bottom).toBe('auto')
  })

  it('flips above when space below is tight', () => {
    const style = computePortalListboxStyle({
      anchor: rect(40, 820, 200, 48),
      viewport: { width: 390, height: 844, bottomInset: 72 },
      preferredMaxHeight: 280,
      gap: 8,
    })
    expect(style.top).toBe('auto')
    expect(style.bottom).toBe(844 - 820 + 8)
  })

  it('clamps horizontal position inside the viewport', () => {
    const style = computePortalListboxStyle({
      anchor: rect(2, 100, 400, 48),
      viewport: { width: 390, height: 800 },
    })
    expect(style.left).toBe(8)
    expect(style.width).toBe(374)
  })
})
