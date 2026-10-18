import { describe, expect, it } from 'vitest'
import {
  classifyQuoteSymbol,
  displayUsdPriceChangePct,
  pairStatsUsdField,
  resolveDisplayPairStatsUsdOhlc,
  resolveDisplayTapeLastPriceUsd,
  resolveTapeLastPriceUsd,
} from '../pairPriceUsd'

describe('modern tape USD (#1512)', () => {
  it('prefers genuine indexed factory USD independently of human ratio', () => {
    expect(resolveTapeLastPriceUsd({ priceUsd: '0.982', price: '160.756' })).toBe('0.982')
  })
  it('missing USD has no legacy, peg, ticker or current-oracle fallback', () => {
    expect(resolveTapeLastPriceUsd({ price: '160.756' })).toBeNull()
    expect(resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: '1', price: null })).toBeNull()
  })
  it('inverts indexed factory USD by the matching human ratio exactly once', () => {
    expect(Number(resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: '0.80', price: '80' }))).toBeCloseTo(
      0.01,
      12
    )
  })
  it('retains the separate portfolio/catalog identity classification', () => {
    expect(classifyQuoteSymbol('USDT')).toBe('unknown')
    expect(classifyQuoteSymbol('USDT', null, 'terra1spoof')).toBe('unknown')
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
