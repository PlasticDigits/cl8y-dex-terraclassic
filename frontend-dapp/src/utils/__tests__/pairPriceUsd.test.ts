import { describe, expect, it } from 'vitest'
import type { IndexerPair, IndexerTrade } from '@/types'
import {
  pairStatsUsdField,
  resolveDisplayTapeLastPriceUsd,
  resolveTapeLastPriceUsd,
  resolveTapePriceUsd,
} from '../pairPriceUsd'

const ust1UstrPair: Pick<IndexerPair, 'asset_0' | 'asset_1'> = {
  asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
  asset_1: { symbol: 'USTR', contract_addr: 'terra1ustr', denom: null, decimals: 18 },
}

const rawUstrPrint: IndexerTrade = {
  id: 1,
  pair_address: 'terra1pair',
  block_height: 1,
  block_timestamp: '2026-08-15T00:00:00Z',
  tx_hash: 'AA',
  sender: 'terra1t',
  offer_asset: 'UST1',
  ask_asset: 'USTR',
  offer_amount: '116624',
  return_amount: '9297047794755092035',
  price: '79718100000000',
}

describe('resolveTapeLastPriceUsd', () => {
  it('prefers indexer price_usd and never uses raw tape as dollars', () => {
    expect(
      resolveTapeLastPriceUsd({
        priceUsd: '0.982',
        price: '79718100000000',
        decimalsBase: 6,
        decimalsQuote: 18,
        quoteSymbol: 'USTR',
        ustcUsd: '0.004928',
      })
    ).toBe('0.982')
  })

  it('18/6 fixture: scales raw tape then × USTR USD (~$0.98)', () => {
    const usd = resolveTapeLastPriceUsd({
      priceUsd: null,
      price: '79718100000000',
      decimalsBase: 6,
      decimalsQuote: 18,
      quoteSymbol: 'USTR',
      ustcUsd: '0.004928',
    })
    expect(usd).not.toBeNull()
    const n = parseFloat(usd!)
    expect(n).toBeGreaterThan(0.9)
    expect(n).toBeLessThan(1.1)
    expect(n).not.toBeGreaterThan(1000)
  })

  it('6/6 fixture: human ~206 cUSTC/UST1 × USTC ≈ $1', () => {
    const usd = resolveTapeLastPriceUsd({
      priceUsd: null,
      price: '206.62',
      decimalsBase: 6,
      decimalsQuote: 6,
      quoteSymbol: 'cUSTC',
      ustcUsd: '0.004928',
    })
    expect(usd).not.toBeNull()
    const n = parseFloat(usd!)
    expect(n).toBeGreaterThan(0.95)
    expect(n).toBeLessThan(1.1)
  })

  it('does not treat raw price as USD when quote catalog is unknown', () => {
    expect(
      resolveTapeLastPriceUsd({
        priceUsd: null,
        price: '79718100000000',
        decimalsBase: 6,
        decimalsQuote: 18,
        quoteSymbol: 'CL8Y',
        ustcUsd: '0.004928',
      })
    ).toBeNull()
  })
})

describe('resolveTapePriceUsd', () => {
  it('uses amount-based human ratio × quote USD for 18/6 print', () => {
    const usd = resolveTapePriceUsd({
      trade: rawUstrPrint,
      pair: ust1UstrPair,
      ustcUsd: '0.004928',
    })
    expect(usd).not.toBeNull()
    const n = parseFloat(usd!)
    expect(n).toBeGreaterThan(0.9)
    expect(n).toBeLessThan(1.1)
  })
})

describe('resolveDisplayTapeLastPriceUsd (#524)', () => {
  it('returns factory USD when not inverted', () => {
    expect(
      resolveDisplayTapeLastPriceUsd({
        inverted: false,
        priceUsd: '0.982',
        price: '206.62',
        quoteSymbol: 'cUSTC',
        ustcUsd: '0.004928',
      })
    ).toBe('0.982')
  })

  it('inverts factory ~$1 via human ~206 to cUSTC dollars', () => {
    const usd = resolveDisplayTapeLastPriceUsd({
      inverted: true,
      priceUsd: '1',
      price: '206',
      quoteSymbol: 'cUSTC',
      displayBaseSymbol: 'cUSTC',
      ustcUsd: '0.004928',
    })
    expect(usd).not.toBeNull()
    expect(parseFloat(usd!)).toBeCloseTo(1 / 206, 8)
  })

  it('falls back to USTC catalog when human price is missing', () => {
    const usd = resolveDisplayTapeLastPriceUsd({
      inverted: true,
      priceUsd: '1',
      price: null,
      quoteSymbol: 'cUSTC',
      displayBaseSymbol: 'cUSTC',
      ustcUsd: '0.004928',
    })
    expect(usd).toBe('0.004928')
  })

  it('inverts factory USD via human to hub USTR, not $1', () => {
    const usd = resolveDisplayTapeLastPriceUsd({
      inverted: true,
      priceUsd: '0.80',
      price: '80',
      quoteSymbol: 'USTR',
      displayBaseSymbol: 'USTR',
      ustcUsd: '0.004928',
    })
    expect(usd).not.toBeNull()
    expect(parseFloat(usd!)).toBeCloseTo(0.8 / 80, 8)
    expect(parseFloat(usd!)).not.toBeCloseTo(1, 2)
    expect(parseFloat(usd!)).not.toBeCloseTo(2.5 * 0.004928, 4)
  })
})

describe('pairStatsUsdField', () => {
  it('returns USD strings and ignores empty', () => {
    expect(pairStatsUsdField('1.02')).toBe('1.02')
    expect(pairStatsUsdField(null)).toBeNull()
    expect(pairStatsUsdField('')).toBeNull()
  })
})
