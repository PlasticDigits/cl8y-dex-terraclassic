import { afterEach, describe, expect, it } from 'vitest'
import type { ChartCandlePoint } from '@/components/charts/priceChartCandles'
import {
  assertSecondaryMarketCopy,
  MAINNET_CUSTC_TOKEN_ADDRESS,
  MAINNET_UST1_TOKEN_ADDRESS,
} from '@/utils/ust1SecondaryMarket'
import {
  defaultDisplayInverted,
  displayPairAssets,
  displayPriceToFactoryToken1PerToken0,
  displaySideFromFactory,
  factorySideFromDisplay,
  factoryToken1PerToken0ToDisplayPrice,
  invertFinitePositive,
  invertOhlc,
  invertUsd,
  isUst1Leg,
  pairDisplayInvertAriaLabel,
  pairDisplayPillLabel,
  parseFinitePositive,
  readStoredPairDisplayInverted,
  resolvePairDisplayInverted,
  writeStoredPairDisplayInverted,
} from '../tradePairDisplayOrientation'

const UST1 = { symbol: 'UST1', contractAddr: MAINNET_UST1_TOKEN_ADDRESS }
const CUSTC = { symbol: 'cUSTC', contractAddr: MAINNET_CUSTC_TOKEN_ADDRESS }
const USTR = { symbol: 'USTR', contractAddr: 'terra1ustrxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }
const VFDUSD = { symbol: 'vFDUSD', contractAddr: 'terra1vfdusdxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }
const CL8Y = { symbol: 'CL8Y', contractAddr: 'terra1cl8yxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }
const CLUNC = { symbol: 'cLUNC', contractAddr: 'terra1cluncxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }

afterEach(() => {
  sessionStorage.clear()
})

describe('isUst1Leg', () => {
  it('matches exact UST1 after trim/casefold', () => {
    expect(isUst1Leg({ symbol: 'UST1' })).toBe(true)
    expect(isUst1Leg({ symbol: 'ust1' })).toBe(true)
    expect(isUst1Leg({ symbol: 'Ust1 ' })).toBe(true)
  })

  it('does not substring-match cUSTC or cUST1', () => {
    expect(isUst1Leg({ symbol: 'cUSTC' })).toBe(false)
    expect(isUst1Leg({ symbol: 'custc' })).toBe(false)
    expect(isUst1Leg({ symbol: 'cUST1' })).toBe(false)
    expect(isUst1Leg({ symbol: 'UST1C' })).toBe(false)
  })

  it('prefers the UST1 contract allowlist', () => {
    expect(isUst1Leg({ symbol: 'nope', contractAddr: MAINNET_UST1_TOKEN_ADDRESS })).toBe(true)
  })

  it('rejects spoofed UST1 symbol on a known non-UST1 contract (H6)', () => {
    expect(isUst1Leg({ symbol: 'UST1', contractAddr: MAINNET_CUSTC_TOKEN_ADDRESS })).toBe(false)
  })
})

describe('defaultDisplayInverted', () => {
  it('inverts UST1/cUSTC, UST1/USTR, UST1/vFDUSD', () => {
    expect(defaultDisplayInverted(UST1, CUSTC)).toBe(true)
    expect(defaultDisplayInverted(UST1, USTR)).toBe(true)
    expect(defaultDisplayInverted(UST1, VFDUSD)).toBe(true)
  })

  it('does not invert when UST1 is already quote', () => {
    expect(defaultDisplayInverted(CUSTC, UST1)).toBe(false)
  })

  it('does not invert non-UST1 pairs or both-UST1 / unknown', () => {
    expect(defaultDisplayInverted(CL8Y, CLUNC)).toBe(false)
    expect(defaultDisplayInverted(UST1, UST1)).toBe(false)
    expect(defaultDisplayInverted(null, CUSTC)).toBe(false)
    expect(defaultDisplayInverted({ symbol: 'mystery' }, { symbol: 'other' })).toBe(false)
  })
})

describe('displayPairAssets + pill', () => {
  it('swaps labels when inverted', () => {
    expect(displayPairAssets('UST1', 'cUSTC', true)).toEqual({ displayBase: 'cUSTC', displayQuote: 'UST1' })
    expect(displayPairAssets('UST1', 'cUSTC', false)).toEqual({ displayBase: 'UST1', displayQuote: 'cUSTC' })
    expect(pairDisplayPillLabel('cUSTC', 'UST1')).toBe('cUSTC/UST1')
    expect(pairDisplayInvertAriaLabel('cUSTC', 'UST1')).toBe('Show UST1 / cUSTC pricing')
    expect(() => assertSecondaryMarketCopy(pairDisplayInvertAriaLabel('cUSTC', 'UST1'))).not.toThrow()
  })
})

describe('invertUsd', () => {
  it('factory ~$1 and human ~206 → displayed USD ≈ 1/206', () => {
    const usd = invertUsd('1', '206')
    expect(usd).not.toBeNull()
    const n = parseFloat(usd!)
    expect(n).toBeCloseTo(1 / 206, 8)
  })

  it('inverted-back equals factory price_usd within float tolerance', () => {
    const factory = 0.982
    const human = 206.62
    const display = invertUsd(factory, human)
    expect(display).not.toBeNull()
    const back = parseFloat(display!) * human
    expect(back).toBeCloseTo(factory, 5)
  })

  it('drops non-finite and non-positive', () => {
    expect(invertUsd('0', '206')).toBeNull()
    expect(invertUsd('1', '0')).toBeNull()
    expect(invertUsd('NaN', '206')).toBeNull()
    expect(invertUsd('1', 'Infinity')).toBeNull()
  })
})

describe('invertOhlc', () => {
  it('reciprocates and swaps high/low', () => {
    const pts: ChartCandlePoint[] = [{ time: 1 as never, open: 2, high: 4, low: 1, close: 3 }]
    const out = invertOhlc(pts)
    expect(out).toHaveLength(1)
    expect(out[0].open).toBeCloseTo(0.5)
    expect(out[0].close).toBeCloseTo(1 / 3)
    expect(out[0].high).toBeCloseTo(1)
    expect(out[0].low).toBeCloseTo(0.25)
    expect(out[0].high).toBeGreaterThanOrEqual(out[0].low)
  })

  it('drops 0, NaN, Infinity (H5)', () => {
    expect(invertOhlc([{ time: 1 as never, open: 0, high: 1, low: 1, close: 1 }])).toEqual([])
    expect(invertOhlc([{ time: 1 as never, open: Number.NaN, high: 1, low: 1, close: 1 }])).toEqual([])
    expect(invertOhlc([{ time: 1 as never, open: Infinity, high: 1, low: 1, close: 1 }])).toEqual([])
    expect(invertOhlc([{ time: 1 as never, open: 1e-18, high: 1e-18, low: 0, close: 1e-18 }])).toEqual([])
  })
})

describe('side mapping', () => {
  it('UST1=asset_0 inverted: display Buy → factory ask; Sell → bid', () => {
    expect(factorySideFromDisplay('bid', true)).toBe('ask')
    expect(factorySideFromDisplay('ask', true)).toBe('bid')
  })

  it('non-inverted: Buy → bid', () => {
    expect(factorySideFromDisplay('bid', false)).toBe('bid')
    expect(factorySideFromDisplay('ask', false)).toBe('ask')
  })

  it('round-trips factory ↔ display', () => {
    expect(displaySideFromFactory(factorySideFromDisplay('bid', true), true)).toBe('bid')
    expect(displaySideFromFactory('bid', false)).toBe('bid')
  })
})

describe('price convert', () => {
  it('display 0.00485 UST1 per cUSTC → factory ≈ 206.1856', () => {
    const factory = displayPriceToFactoryToken1PerToken0('0.00485', true)
    expect(factory).not.toBeNull()
    expect(parseFloat(factory!)).toBeCloseTo(1 / 0.00485, 6)
  })

  it('reverse convert for book prefill', () => {
    const display = factoryToken1PerToken0ToDisplayPrice('206.2', true)
    expect(display).not.toBeNull()
    expect(parseFloat(display!)).toBeCloseTo(1 / 206.2, 8)
    const back = displayPriceToFactoryToken1PerToken0(display, true)
    expect(parseFloat(back!)).toBeCloseTo(206.2, 6)
  })

  it('non-inverted is identity', () => {
    expect(parseFloat(displayPriceToFactoryToken1PerToken0('2.5', false)!)).toBeCloseTo(2.5)
  })

  it('mixed 6/18-style ratio stays finite (H1)', () => {
    const factory = displayPriceToFactoryToken1PerToken0('0.000000000012', true)
    expect(factory).not.toBeNull()
    expect(Number.isFinite(parseFloat(factory!))).toBe(true)
    expect(parseFloat(factory!)).toBeGreaterThan(0)
  })
})

describe('parseFinitePositive / invertFinitePositive', () => {
  it('rejects junk', () => {
    expect(parseFinitePositive('')).toBeNull()
    expect(parseFinitePositive('0')).toBeNull()
    expect(invertFinitePositive('0')).toBeNull()
    expect(invertFinitePositive(1e-320)).toBeNull()
  })
})

describe('sessionStorage persistence', () => {
  it('reads product default when unset; keeps per-pair choice', () => {
    const a = 'terra1pairaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
    const b = 'terra1pairbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
    expect(resolvePairDisplayInverted(a, UST1, CUSTC)).toBe(true)
    writeStoredPairDisplayInverted(a, false)
    expect(readStoredPairDisplayInverted(a)).toBe(false)
    expect(resolvePairDisplayInverted(a, UST1, CUSTC)).toBe(false)
    expect(resolvePairDisplayInverted(b, UST1, CUSTC)).toBe(true)
    expect(resolvePairDisplayInverted(b, CL8Y, CLUNC)).toBe(false)
  })
})
