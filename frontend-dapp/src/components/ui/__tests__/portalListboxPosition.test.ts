import { describe, expect, it } from 'vitest'
import {
  computePortalListboxStyle,
  PORTAL_LISTBOX_VIEWPORT_PAD,
  portalListboxReservedTop,
} from '../portalListboxPosition'
import { readPortalListboxViewport } from '@/lib/portalListboxViewport'

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

function menuBottom(style: { top?: string | number; maxHeight?: string | number }): number {
  expect(typeof style.top).toBe('number')
  expect(typeof style.maxHeight).toBe('number')
  return (style.top as number) + (style.maxHeight as number)
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

  it('keeps the menu above tab + in-app + finger reserve on a short visual viewport (GitLab #632)', () => {
    const viewport = readPortalListboxViewport({
      innerWidth: 390,
      innerHeight: 844,
      visualViewport: { width: 390, height: 400, offsetTop: 0, offsetLeft: 0 },
      tabBarInset: 56,
      inAppChrome: true,
      coarseNarrow: true,
    })
    const style = computePortalListboxStyle({
      anchor: rect(16, 80, 358, 48),
      viewport,
      preferredMaxHeight: 240,
      gap: 8,
    })
    const reserved = portalListboxReservedTop(viewport)
    expect(menuBottom(style)).toBeLessThanOrEqual(reserved - PORTAL_LISTBOX_VIEWPORT_PAD)
    expect(style.maxHeight).toBeGreaterThan(0)
  })

  it('flips above when space-below collapses under the reserved band (GitLab #632)', () => {
    const viewport = readPortalListboxViewport({
      innerWidth: 360,
      innerHeight: 640,
      visualViewport: { width: 360, height: 400, offsetTop: 0, offsetLeft: 0 },
      tabBarInset: 56,
      inAppChrome: true,
      coarseNarrow: true,
    })
    const style = computePortalListboxStyle({
      anchor: rect(12, 360, 336, 48),
      viewport,
      preferredMaxHeight: 240,
      gap: 8,
    })
    expect(style.top).toBe('auto')
    expect(typeof style.maxHeight).toBe('number')
    expect((style.maxHeight as number) + PORTAL_LISTBOX_VIEWPORT_PAD).toBeLessThanOrEqual(360)
  })

  it('does not force 120px maxHeight into the reserved band', () => {
    const style = computePortalListboxStyle({
      anchor: rect(16, 40, 300, 48),
      viewport: { width: 390, height: 400, bottomInset: 250 },
      preferredMaxHeight: 240,
      gap: 8,
    })
    expect(style.top).toBe(96)
    expect(style.maxHeight).toBe(400 - 88 - 8 - 8 - 250)
    expect(menuBottom(style)).toBeLessThanOrEqual(400 - 250 - PORTAL_LISTBOX_VIEWPORT_PAD)
  })

  it('clamps horizontally on a narrow visual viewport with offsetLeft', () => {
    const style = computePortalListboxStyle({
      anchor: rect(2, 100, 400, 48),
      viewport: { width: 360, height: 640, offsetLeft: 8 },
    })
    expect(style.left).toBe(16)
    expect(style.width).toBe(344)
  })
})
