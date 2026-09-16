import { describe, expect, it } from 'vitest'
import type { IndexerPair, IndexerTrade } from '@/types'
import {
  classifyQuoteSymbol,
  displayUsdPriceChangePct,
  pairStatsUsdField,
  quoteTokenUsd,
  resolveDisplayPairStatsUsdOhlc,
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

describe('resolveDisplayPairStatsUsdOhlc (GitLab #680)', () => {
  it('keeps factory USD and factory % when not inverted', () => {
    const out = resolveDisplayPairStatsUsdOhlc({
      inverted: false,
      highUsd: '1.02',
      lowUsd: '0.98',
      openUsd: '1.00',
      closeUsd: '1.01',
      highHuman: '210',
      lowHuman: '202',
      openHuman: '206',
      closeHuman: '208',
      factoryPriceChangePct: 1.0,
    })
    expect(out.highUsd).toBe('1.02')
    expect(out.lowUsd).toBe('0.98')
    expect(out.priceChangePct).toBe(1.0)
  })

  it('inverts via invertUsd and recomputes % — not 1/x of USD and not negated factory %', () => {
    const out = resolveDisplayPairStatsUsdOhlc({
      inverted: true,
      highUsd: '1.02',
      lowUsd: '0.98',
      openUsd: '1.00',
      closeUsd: '1.01',
      highHuman: '210',
      lowHuman: '202',
      openHuman: '206',
      closeHuman: '208',
      factoryPriceChangePct: 1.0,
    })
    expect(parseFloat(out.openUsd!)).toBeCloseTo(1 / 206, 8)
    expect(parseFloat(out.closeUsd!)).toBeCloseTo(1.01 / 208, 8)
    expect(parseFloat(out.highUsd!)).toBeGreaterThanOrEqual(parseFloat(out.lowUsd!))
    expect(out.priceChangePct).not.toBe(-1)
    expect(out.priceChangePct).not.toBe(1)
    expect(out.priceChangePct).toBeCloseTo(displayUsdPriceChangePct(out.openUsd, out.closeUsd)!, 8)
    expect(parseFloat(out.openUsd!)).not.toBeCloseTo(1, 2)
  })
})

describe('USDT registry pin (#1258)', () => {
  const pin = 'terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4'

  it('classifies only the pinned CW20 as usdt; ticker spoofs stay unknown', () => {
    expect(classifyQuoteSymbol('USDT', null, pin)).toBe('usdt')
    expect(classifyQuoteSymbol('usdt', null, pin.toUpperCase())).toBe('usdt')
    expect(classifyQuoteSymbol('USDT')).toBe('unknown')
    expect(classifyQuoteSymbol('USDT', null, 'terra1spoofusdtxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx')).toBe('unknown')
    expect(classifyQuoteSymbol('VFDUSD')).toBe('unknown')
    expect(classifyQuoteSymbol('FDUSD')).toBe('unknown')
    expect(classifyQuoteSymbol('CL8Y')).toBe('unknown')
    expect(quoteTokenUsd('usdt', 0.8)).toBe(1)
    expect(quoteTokenUsd('peg1', 0.8)).toBe(1)
  })

  it('legacy fallback: human USDT-per-cLUNC × $1 when indexer price_usd is missing', () => {
    // Pre-#522 indexer stored raw quote-per-base; 6/18 scale is 10^(6−18).
    const usd = resolveTapeLastPriceUsd({
      priceUsd: null,
      price: '41230000',
      decimalsBase: 6,
      decimalsQuote: 18,
      quoteSymbol: 'USDT',
      quoteContract: pin,
    })
    expect(usd).not.toBeNull()
    expect(parseFloat(usd!)).toBeCloseTo(0.00004123, 12)
    expect(usd).not.toMatch(/[TMB]/)
  })

  it('does not invent dollars for a USDT ticker on another contract', () => {
    expect(
      resolveTapeLastPriceUsd({
        priceUsd: null,
        price: '0.00004123',
        decimalsBase: 6,
        decimalsQuote: 18,
        quoteSymbol: 'USDT',
        quoteContract: 'terra1notthepinxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      })
    ).toBeNull()
  })

  it('prefers indexer price_usd over the advisory $1 fallback', () => {
    expect(
      resolveTapeLastPriceUsd({
        priceUsd: '0.00004123',
        price: '999',
        decimalsBase: 6,
        decimalsQuote: 18,
        quoteSymbol: 'USDT',
        quoteContract: pin,
      })
    ).toBe('0.00004123')
  })

  it('Charts 24h USD OHLC binds indexer *_usd (not human tape, not em-dash)', () => {
    const out = resolveDisplayPairStatsUsdOhlc({
      inverted: false,
      highUsd: '0.00005',
      lowUsd: '0.00004',
      openUsd: '0.000041',
      closeUsd: '0.000042',
      factoryPriceChangePct: 2.4,
    })
    expect(out.highUsd).toBe('0.00005')
    expect(out.lowUsd).toBe('0.00004')
    expect(out.openUsd).toBe('0.000041')
    expect(out.closeUsd).toBe('0.000042')
    expect(out.priceChangePct).toBe(2.4)
    expect(pairStatsUsdField(null)).toBeNull()
  })

  it('?price= invert uses invertUsd (not 1/x of USD) when USDT is quote at $1', () => {
    const out = resolveDisplayPairStatsUsdOhlc({
      inverted: true,
      highUsd: '0.000042',
      lowUsd: '0.000040',
      openUsd: '0.000041',
      closeUsd: '0.0000415',
      highHuman: '0.000042',
      lowHuman: '0.000040',
      openHuman: '0.000041',
      closeHuman: '0.0000415',
      factoryPriceChangePct: 1.2,
    })
    expect(parseFloat(out.openUsd!)).toBeCloseTo(1, 6)
    expect(parseFloat(out.closeUsd!)).toBeCloseTo(1, 6)
    expect(parseFloat(out.highUsd!)).toBeGreaterThanOrEqual(parseFloat(out.lowUsd!))
    expect(parseFloat(out.openUsd!)).not.toBeCloseTo(1 / 0.000041, 0)
    expect(out.priceChangePct).toBeCloseTo(displayUsdPriceChangePct(out.openUsd, out.closeUsd)!, 8)
  })
})
