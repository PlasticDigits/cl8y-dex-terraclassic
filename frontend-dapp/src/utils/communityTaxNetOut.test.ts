import { describe, expect, it } from 'vitest'
import { applyBuyTaxNet, displayReceiveNet, effectiveBuyTaxBps, withBuyTaxReceiveDisplay } from './communityTaxNetOut'

describe('communityTaxNetOut (#615)', () => {
  it('matches TaxPreview floor buy split', () => {
    expect(applyBuyTaxNet('1000000', 100)).toBe('990000')
    expect(applyBuyTaxNet('10000', 1)).toBe('9999')
    expect(applyBuyTaxNet('1', 100)).toBe('1')
    expect(applyBuyTaxNet('50', 0)).toBe('50')
    expect(applyBuyTaxNet('50', null)).toBe('50')
  })

  it('exempt trader is 0 bps; unknown exempt fail-closed', () => {
    expect(effectiveBuyTaxBps(250, true)).toBe(0)
    expect(effectiveBuyTaxBps(250, false)).toBe(250)
    expect(effectiveBuyTaxBps(250, null)).toBe(250)
    expect(effectiveBuyTaxBps(250, undefined)).toBe(250)
    expect(displayReceiveNet('10000', effectiveBuyTaxBps(250, true))).toBe('10000')
    expect(displayReceiveNet('10000', effectiveBuyTaxBps(250, false))).toBe('9750')
  })

  it('pair-direct helper keeps pre-tax execute amount', () => {
    const out = withBuyTaxReceiveDisplay({ return_amount: '1000000' }, 100)
    expect(out.return_amount).toBe('990000')
    expect(out.executeAmountOut).toBe('1000000')
  })

  it('does not resize invalid / empty raw', () => {
    expect(applyBuyTaxNet('', 100)).toBe('')
    expect(applyBuyTaxNet('1.5', 100)).toBe('1.5')
  })
})
