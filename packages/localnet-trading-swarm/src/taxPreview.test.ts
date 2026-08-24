import { describe, expect, it } from 'vitest'
import {
  balanceCoversDebit,
  failClosedSellDebit,
  requiredWalletDebit,
  taxLogFields,
} from './taxPreview.js'

describe('failClosedSellDebit', () => {
  it('adds floor(amount * 500 / 10000)', () => {
    expect(failClosedSellDebit(10_000n, 500)).toBe(10_500n)
    expect(failClosedSellDebit(1_000_000n, 500)).toBe(1_050_000n)
  })

  it('is identity at 0 bps', () => {
    expect(failClosedSellDebit(99n, 0)).toBe(99n)
  })
})

describe('requiredWalletDebit', () => {
  it('pair-direct uses TaxPreview.debit', () => {
    const preview = {
      declared: '10000',
      debit: '10500',
      credit: '10000',
      tax: '500',
    }
    expect(requiredWalletDebit(preview, 10_000n, 'pair', 500)).toBe(10_500n)
  })

  it('router adds hop_trader_debit to the 1:1 Send debit', () => {
    const preview = {
      declared: '10000',
      debit: '10000',
      credit: '10000',
      tax: '500',
      hop_trader: 'terra1bot',
      hop_trader_debit: '500',
    }
    expect(requiredWalletDebit(preview, 10_000n, 'router', 500)).toBe(10_500n)
  })

  it('router without hop extra-debit fail-closes', () => {
    const preview = {
      declared: '10000',
      debit: '10000',
      credit: '10000',
      tax: '0',
    }
    expect(requiredWalletDebit(preview, 10_000n, 'router', 500)).toBe(10_500n)
  })

  it('missing preview fail-closes on both paths', () => {
    expect(requiredWalletDebit(null, 10_000n, 'pair', 500)).toBe(10_500n)
    expect(requiredWalletDebit(undefined, 20_000n, 'router', 500)).toBe(21_000n)
  })

  it('refuses a Send sized as 100% of balance (no tax headroom)', () => {
    const debit = failClosedSellDebit(10_000n, 500)
    expect(balanceCoversDebit(10_000n, debit)).toBe(false)
    expect(balanceCoversDebit(10_500n, debit)).toBe(true)
  })
})

describe('taxLogFields', () => {
  it('emits structured debit/credit/bps/path', () => {
    const log = taxLogFields(
      { declared: '100', debit: '105', credit: '100', tax: '5' },
      100n,
      'pair',
      500
    )
    expect(log).toEqual({
      tax_debit: '105',
      tax_credit: '100',
      bps: 500,
      path: 'pair',
    })
  })
})
