import { describe, expect, it } from 'vitest'
import {
  COMMUNITY_TAX_INVOICE_UST1_RAW,
  COMMUNITY_TAX_MAX_BPS,
  COMMUNITY_TAX_SKUS,
  instantiateTaxCaps,
  isUnlockableAfterCreate,
  parseTaxBps,
  skuInvoiceUst1Raw,
  skuInvoiceUst1RawString,
} from './communityTaxSku'

describe('communityTaxSku (#593)', () => {
  it('SKU total: 0 / 1 / 3 → 0 / 50 / 150 UST1', () => {
    expect(skuInvoiceUst1Raw(0)).toBe(0n)
    expect(skuInvoiceUst1Raw(1)).toBe(COMMUNITY_TAX_INVOICE_UST1_RAW)
    expect(skuInvoiceUst1Raw(3)).toBe(150_000_000n)
    expect(skuInvoiceUst1RawString(3)).toBe('150000000')
  })

  it('Minting cannot be purchased after create', () => {
    expect(isUnlockableAfterCreate('mint_control')).toBe(false)
    expect(isUnlockableAfterCreate('transfer_tax')).toBe(true)
    expect(COMMUNITY_TAX_SKUS.find((s) => s.id === 'mint_control')?.createOnly).toBe(true)
  })

  it('uses retail labels, not raw SKU JSON', () => {
    expect(COMMUNITY_TAX_SKUS.map((s) => s.label)).toContain('Minting')
    expect(COMMUNITY_TAX_SKUS.map((s) => s.label)).not.toContain('mint_control')
  })

  it('rejects tax bps above on-chain max', () => {
    expect(parseTaxBps('2500')).toEqual({ ok: true, bps: 2500 })
    expect(parseTaxBps('2501').ok).toBe(false)
    expect(parseTaxBps('abc').ok).toBe(false)
    expect(parseTaxBps('')).toEqual({ ok: true, bps: 0 })
  })

  it('instantiate caps never sum above 2500 (not 2500+2500+2500)', () => {
    const locked = instantiateTaxCaps({
      buyBps: 100,
      sellBps: 200,
      variableRates: false,
      transferTax: false,
    })
    expect(locked).toEqual({ maxBuyBps: 100, maxSellBps: 200, maxTransferBps: 0 })
    expect(locked.maxBuyBps + locked.maxSellBps + locked.maxTransferBps).toBeLessThanOrEqual(COMMUNITY_TAX_MAX_BPS)

    const variable = instantiateTaxCaps({
      buyBps: 0,
      sellBps: 0,
      variableRates: true,
      transferTax: true,
    })
    expect(variable.maxBuyBps + variable.maxSellBps + variable.maxTransferBps).toBe(COMMUNITY_TAX_MAX_BPS)
  })
})
