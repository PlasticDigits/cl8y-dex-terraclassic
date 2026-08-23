import { describe, expect, it } from 'vitest'
import {
  applyExtraDebitSellCap,
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  COMMUNITY_TAX_PAIR_DIRECT_COPY,
  effectiveExtraDebitSellBps,
  extraDebitSellBpsForExecute,
  extraDebitSellHuman,
  maxDeclaredForExtraDebitSell,
  ROUTER_TAX_SKIP_HINT,
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

  it('#609 manager-exempt skips extra-debit; unknown stays fail-closed', () => {
    expect(effectiveExtraDebitSellBps(500, true)).toBe(0)
    expect(effectiveExtraDebitSellBps(500, false)).toBe(500)
    expect(effectiveExtraDebitSellBps(500, null)).toBe(500)
    expect(effectiveExtraDebitSellBps(500, undefined)).toBe(500)
    expect(effectiveExtraDebitSellBps(null, true)).toBeNull()
    expect(applyExtraDebitSellCap(10_000_000n, effectiveExtraDebitSellBps(500, true))).toBe(10_000_000n)
  })

  it('router hops are Honest: skip extra-debit Max and disclose (#607)', () => {
    expect(communityTaxExecuteUsesRouter(1)).toBe(false)
    expect(communityTaxExecuteUsesRouter(2)).toBe(true)
    expect(communityTaxExecuteUsesRouter(1, true)).toBe(true)
    expect(extraDebitSellBpsForExecute(500, false)).toBe(500)
    expect(extraDebitSellBpsForExecute(500, true)).toBeNull()
    expect(communityTaxRouteHint({ payIsTax: true, usesRouter: false, sellBps: 500 })).toBe(SELL_TAX_EXTRA_HINT)
    expect(communityTaxRouteHint({ payIsTax: true, usesRouter: true, sellBps: 500 })).toBe(ROUTER_TAX_SKIP_HINT)
    expect(communityTaxRouteHint({ payIsTax: false, receiveIsTax: true, usesRouter: true })).toBe(ROUTER_TAX_SKIP_HINT)
    expect(COMMUNITY_TAX_PAIR_DIRECT_COPY).toBe('Buy/sell tax is pair-direct only.')
  })
})
