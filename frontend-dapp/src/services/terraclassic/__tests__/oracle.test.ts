import { describe, it, expect } from 'vitest'
import { computeTwapPrice, computeTwapPriceDecimalString } from '../oracle'

const E18 = 10n ** 18n

describe('computeTwapPriceDecimalString (GitLab #564)', () => {
  it('returns arithmetic raw Decimal as a decimal string', () => {
    expect(computeTwapPriceDecimalString(0n, 2n * E18, 1)).toBe('2')
    expect(computeTwapPriceDecimalString(0n, E18 / 2n, 1)).toBe('0.5')
  })

  it('returns null for inverted cum, zero window, or zero average', () => {
    expect(computeTwapPriceDecimalString(10n, 5n, 1)).toBeNull()
    expect(computeTwapPriceDecimalString(0n, E18, 0)).toBeNull()
    expect(computeTwapPriceDecimalString(0n, 0n, 1)).toBeNull()
  })

  it('computeTwapPrice still returns 0 on inverted cum', () => {
    expect(computeTwapPrice(10n, 5n, 1)).toBe(0)
  })
})
