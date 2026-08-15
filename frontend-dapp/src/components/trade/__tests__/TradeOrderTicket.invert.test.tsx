import { describe, expect, it } from 'vitest'
import { displayPriceToFactoryToken1PerToken0, factorySideFromDisplay } from '@/utils/tradePairDisplayOrientation'

describe('TradeOrderTicket invert submit mapping (GitLab #524 / H1)', () => {
  it('inverted Buy cUSTC submits factory ask at reciprocal token1/token0, not 1/P as-is', () => {
    const displayPrice = '0.00485'
    const factoryPrice = displayPriceToFactoryToken1PerToken0(displayPrice, true)
    const factorySide = factorySideFromDisplay('bid', true)
    expect(factorySide).toBe('ask')
    expect(factoryPrice).not.toBe(displayPrice)
    expect(parseFloat(factoryPrice!)).toBeCloseTo(1 / 0.00485, 6)
    expect(parseFloat(factoryPrice!)).toBeGreaterThan(1)
  })

  it('non-inverted Buy UST1 submits factory bid at the typed price', () => {
    expect(factorySideFromDisplay('bid', false)).toBe('bid')
    expect(parseFloat(displayPriceToFactoryToken1PerToken0('206.2', false)!)).toBeCloseTo(206.2)
  })
})
