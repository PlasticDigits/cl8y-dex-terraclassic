import { describe, it, expect } from 'vitest'
import { effectiveSwapFeeBps, makerPlacementFeeBps } from '../limitOrderFeeSummary'

describe('limitOrderFeeSummary', () => {
  it('effectiveSwapFeeBps matches integer pair discount formula', () => {
    expect(effectiveSwapFeeBps(30, 0)).toBe(30)
    expect(effectiveSwapFeeBps(30, undefined)).toBe(30)
    expect(effectiveSwapFeeBps(30, 5000)).toBe(15)
    expect(effectiveSwapFeeBps(100, 2500)).toBe(75)
  })

  it('makerPlacementFeeBps floors half of effective', () => {
    expect(makerPlacementFeeBps(30)).toBe(15)
    expect(makerPlacementFeeBps(31)).toBe(15)
    expect(makerPlacementFeeBps(1)).toBe(0)
    expect(makerPlacementFeeBps(0)).toBe(0)
  })
})
