import { describe, expect, it } from 'vitest'
import {
  humanLimitPriceToRaw,
  limitPriceDecimalsFromPair,
  rawLimitPriceToHuman,
  scaleDecimalStringByPow10,
  scaleHumanLimitPriceForChain,
  scaleRawLimitPriceForDisplay,
} from '@/utils/limitOrderPriceScale'

describe('scaleDecimalStringByPow10', () => {
  it('multiplies 78.76 by 10^12 for UST1/USTR raw', () => {
    expect(scaleDecimalStringByPow10('78.76', 12)).toBe('78760000000000')
  })

  it('divides raw 7.876e13 back to 78.76', () => {
    expect(scaleDecimalStringByPow10('78760000000000', -12)).toBe('78.76')
  })

  it('scales reverse orientation 0.0127 by 10^-12', () => {
    expect(scaleDecimalStringByPow10('0.0127', -12)).toBe('0.0000000000000127')
  })
})

describe('human ↔ raw limit price (GitLab #529)', () => {
  it('UST1(6)/USTR(18) human 78.76 → raw 7.876e13 and back', () => {
    const raw = humanLimitPriceToRaw('78.76', 6, 18)
    expect(raw).toBe('78760000000000')
    expect(rawLimitPriceToHuman(raw, 6, 18)).toBe('78.76')
  })

  it('USTR(18)/UST1(6) human 0.0127 → tiny raw and back', () => {
    const raw = humanLimitPriceToRaw('0.0127', 18, 6)
    expect(raw).toBe('0.0000000000000127')
    expect(rawLimitPriceToHuman(raw, 18, 6)).toBe('0.0127')
  })

  it('equal decimals is a no-op', () => {
    expect(humanLimitPriceToRaw('1.5', 6, 6)).toBe('1.5')
    expect(scaleHumanLimitPriceForChain('1.5', { decimals0: 6, decimals1: 6 })).toBe('1.5')
    expect(scaleRawLimitPriceForDisplay('1.5', { decimals0: 6, decimals1: 6 })).toBe('1.5')
  })

  it('omitted scale leaves the string unchanged', () => {
    expect(scaleHumanLimitPriceForChain('79.1', null)).toBe('79.1')
    expect(scaleRawLimitPriceForDisplay('79.1', undefined)).toBe('79.1')
  })

  it('reads decimals from an indexer pair row', () => {
    expect(
      limitPriceDecimalsFromPair({
        asset_0: { decimals: 6 },
        asset_1: { decimals: 18 },
      })
    ).toEqual({ decimals0: 6, decimals1: 18 })
    expect(limitPriceDecimalsFromPair(null)).toBeNull()
  })
})
