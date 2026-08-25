import { describe, expect, it } from 'vitest'
import {
  PORTAL_LISTBOX_FINGER_GAP_PX,
  PORTAL_LISTBOX_IN_APP_CHROME_RESERVE_PX,
  readPortalListboxViewport,
} from '../portalListboxViewport'

describe('readPortalListboxViewport (GitLab #632)', () => {
  it('desktop: no visualViewport, no insets', () => {
    const vp = readPortalListboxViewport({
      innerWidth: 1440,
      innerHeight: 900,
      visualViewport: null,
      tabBarInset: 0,
      inAppChrome: false,
      coarseNarrow: false,
    })
    expect(vp).toEqual({
      width: 1440,
      height: 900,
      offsetLeft: 0,
      topInset: 0,
      bottomInset: 0,
    })
  })

  it('adds IME occluded band from visualViewport + finger gap', () => {
    const vp = readPortalListboxViewport({
      innerWidth: 390,
      innerHeight: 844,
      visualViewport: { width: 390, height: 400, offsetTop: 0, offsetLeft: 0 },
      tabBarInset: 56,
      inAppChrome: false,
      coarseNarrow: true,
    })
    expect(vp.width).toBe(390)
    expect(vp.height).toBe(844)
    expect(vp.topInset).toBe(0)
    expect(vp.bottomInset).toBe(56 + (844 - 400) + PORTAL_LISTBOX_FINGER_GAP_PX)
  })

  it('adds in-app chrome reserve only when in-app and coarse/narrow', () => {
    const inApp = readPortalListboxViewport({
      innerWidth: 360,
      innerHeight: 640,
      visualViewport: { width: 360, height: 640, offsetTop: 0, offsetLeft: 0 },
      tabBarInset: 56,
      inAppChrome: true,
      coarseNarrow: true,
    })
    expect(inApp.bottomInset).toBe(56 + PORTAL_LISTBOX_IN_APP_CHROME_RESERVE_PX + PORTAL_LISTBOX_FINGER_GAP_PX)

    const desktopSpoof = readPortalListboxViewport({
      innerWidth: 1440,
      innerHeight: 900,
      visualViewport: null,
      tabBarInset: 0,
      inAppChrome: true,
      coarseNarrow: false,
    })
    expect(desktopSpoof.bottomInset).toBe(0)
  })

  it('clamps spoofed visualViewport (0 / NaN / huge) to a usable box', () => {
    const vp = readPortalListboxViewport({
      innerWidth: 390,
      innerHeight: 844,
      visualViewport: { width: Number.NaN, height: 0, offsetTop: -20, offsetLeft: 1e9 },
      tabBarInset: 56,
      inAppChrome: true,
      coarseNarrow: true,
    })
    expect(vp.width).toBeGreaterThanOrEqual(1)
    expect(vp.height).toBe(844)
    expect(vp.offsetLeft).toBeLessThanOrEqual(390)
    expect(vp.bottomInset).toBeGreaterThan(56)
    expect(Number.isFinite(vp.bottomInset)).toBe(true)
  })
})
