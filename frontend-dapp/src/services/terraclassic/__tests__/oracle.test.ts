import { describe, it, expect } from 'vitest'
import { computeTwapPrice, computeTwapPriceDecimalString } from '../oracle'

const E18 = 10n ** 18n

describe('computeTwapPriceDecimalString (GitLab #564)', () => {
  it('returns arithmetic raw Decimal as a decimal string', () => {
    expect(computeTwapPriceDecimalString(0n, 2n * E18, 1)).toBe('2')
    expect(computeTwapPriceDecimalString(0n, E18 / 2n, 1)).toBe('0.5')
  })

  it('returns null for inverted cum, zero window, or zero average', () => {
    // end < start is corruption, not a 2^128 wrap (#1322).
    expect(computeTwapPriceDecimalString(10n, 5n, 1)).toBeNull()
    expect(computeTwapPriceDecimalString(0n, E18, 0)).toBeNull()
    expect(computeTwapPriceDecimalString(0n, 0n, 1)).toBeNull()
  })

  it('returns the window integral when the cumulative crosses 2^128 (#1224 / #1322)', () => {
    const start = (1n << 128n) - E18
    const end = (1n << 128n) + E18
    expect(computeTwapPriceDecimalString(start, end, 2)).toBe('1')
    const live = 340144359629112943994362291128760055446n
    const delta = 10n * E18
    expect(computeTwapPriceDecimalString(live, live + delta, 10)).toBe('1')
  })

  it('computeTwapPrice still returns 0 on inverted cum', () => {
    expect(computeTwapPrice(10n, 5n, 1)).toBe(0)
  })
})
