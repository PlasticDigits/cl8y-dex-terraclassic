import { describe, it, expect } from 'vitest'
import {
  effectiveSwapFeeBps,
  makerPlacementFeeBps,
  bpsToPercentLabel,
  resolveLimitDiscountBps,
} from '../limitOrderFeeSummary'

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

  it('resolveLimitDiscountBps prefers explicit limit discount (#514)', () => {
    expect(resolveLimitDiscountBps(9500, 10000)).toBe(10000)
    expect(resolveLimitDiscountBps(9500, null)).toBe(9500)
    expect(resolveLimitDiscountBps(9500, undefined)).toBe(9500)
    expect(makerPlacementFeeBps(effectiveSwapFeeBps(180, resolveLimitDiscountBps(9500, 10000)))).toBe(0)
    expect(makerPlacementFeeBps(effectiveSwapFeeBps(180, 9500))).toBe(4)
  })

  it('bpsToPercentLabel formats retail fee copy (#419)', () => {
    expect(bpsToPercentLabel(15)).toBe('0.15%')
    expect(bpsToPercentLabel(30)).toBe('0.30%')
    expect(bpsToPercentLabel(100)).toBe('1.00%')
  })
})
