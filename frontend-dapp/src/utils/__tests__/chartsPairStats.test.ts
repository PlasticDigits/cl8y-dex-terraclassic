import { describe, it, expect } from 'vitest'
import { formatPairStatsUsdOhlc, formatTwapHumanPrice, twapRawToDecimalString } from '../chartsPairStats'

describe('formatTwapHumanPrice (GitLab #564)', () => {
  it('scales UST1/USTR raw 111.009e12 to human 111.009 class, not T', () => {
    const raw = '111009000000000'
    const human = formatTwapHumanPrice(raw, 6, 18)
    expect(human).toMatch(/^111\.009/)
    expect(human).not.toMatch(/T$/)
    expect(human).not.toMatch(/K$/)
  })

  it('is identity for equal-decimal 6/6', () => {
    expect(formatTwapHumanPrice('1.05', 6, 6)).toMatch(/^1\.05/)
    expect(formatTwapHumanPrice(1.05, 6, 6)).toMatch(/^1\.05/)
    expect(formatTwapHumanPrice('1.05', 6, 6)).not.toMatch(/T$/)
  })

  it('scales 18/6 the other way without compact T', () => {
    const human = formatTwapHumanPrice('0.00000000000111009', 18, 6)
    expect(human).toMatch(/^1\.11009/)
    expect(human).not.toMatch(/T$/)
  })

  it('returns em dash for missing decimals, non-positive, or junk', () => {
    expect(formatTwapHumanPrice('111', undefined, 18)).toBe('—')
    expect(formatTwapHumanPrice('111', 6, 19)).toBe('—')
    expect(formatTwapHumanPrice(0, 6, 18)).toBe('—')
    expect(formatTwapHumanPrice(-1, 6, 6)).toBe('—')
    expect(formatTwapHumanPrice(Number.POSITIVE_INFINITY, 6, 6)).toBe('—')
    expect(formatTwapHumanPrice('abc', 6, 6)).toBe('—')
    expect(formatTwapHumanPrice('<script>', 6, 6)).toBe('—')
    expect(formatTwapHumanPrice(null, 6, 6)).toBe('—')
  })
})

describe('twapRawToDecimalString', () => {
  it('rejects scientific notation strings', () => {
    expect(twapRawToDecimalString('111.009e12')).toBeNull()
  })
})

describe('formatPairStatsUsdOhlc (GitLab #564)', () => {
  it('prints sub-dollar factory USD without compact T/M', () => {
    expect(formatPairStatsUsdOhlc('0.682427')).toBe('$0.682427')
    expect(formatPairStatsUsdOhlc('0.682427')).not.toMatch(/[TMBK]/)
    expect(formatPairStatsUsdOhlc(null)).toBe('—')
    expect(formatPairStatsUsdOhlc('0')).toBe('—')
  })
})
