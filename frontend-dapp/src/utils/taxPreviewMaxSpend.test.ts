import { describe, expect, it } from 'vitest'
import {
  applyExtraDebitSellCap,
  extraDebitSellHuman,
  maxDeclaredForExtraDebitSell,
  SELL_TAX_EXTRA_HINT,
} from './taxPreviewMaxSpend'

describe('taxPreviewMaxSpend (#593 extra-debit sell)', () => {
  it('reduces max so debit fits in balance (5% sell)', () => {
    const balance = 1_000_000n
    const declared = maxDeclaredForExtraDebitSell(balance, 500)
    const tax = (declared * 500n) / 10_000n
    expect(declared + tax).toBeLessThanOrEqual(balance)
    expect(declared).toBeLessThan(balance)
  })

  it('zero sell bps keeps full balance', () => {
    expect(maxDeclaredForExtraDebitSell(99n, 0)).toBe(99n)
    expect(applyExtraDebitSellCap(99n, 0)).toBe(99n)
    expect(applyExtraDebitSellCap(99n, null)).toBe(99n)
  })

  it('human max is not the raw 100% balance when taxed', () => {
    expect(extraDebitSellHuman('1000000', 6, 500)).not.toBe('1')
    expect(SELL_TAX_EXTRA_HINT).toBe('Sell tax extra')
  })

  it('does not offer 100% of a taxed sell (abuse = self-DoS)', () => {
    const balance = 10_000_000n
    expect(applyExtraDebitSellCap(balance, 2500)).toBeLessThan(balance)
  })
})
