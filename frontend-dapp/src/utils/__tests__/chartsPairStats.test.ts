import { describe, it, expect } from 'vitest'
import { formatTokenAmount } from '../formatAmount'
import {
  formatChartsPairTokenVolume,
  formatPairStatsUsdOhlc,
  formatTwapHumanPrice,
  twapRawToDecimalString,
} from '../chartsPairStats'

describe('formatChartsPairTokenVolume (GitLab #565)', () => {
  it('humanizes 6-dec UST1 raw; never compact-formats as 847.0M', () => {
    const shown = formatChartsPairTokenVolume('847004054', 6)
    expect(shown).toBe(formatTokenAmount('847004054', 6))
    expect(shown).not.toMatch(/847\.0M/)
    expect(shown).toMatch(/^847/)
  })

  it('humanizes 18-dec USTR quote; never compact-formats as T', () => {
    const raw = '19300000000000000000'
    const shown = formatChartsPairTokenVolume(raw, 18)
    expect(shown).not.toMatch(/T$/)
    expect(shown).toBe(formatTokenAmount(raw, 18))
  })

  it('does not apply UST1 6-dec to an 18-dec quote raw (decimal lie)', () => {
    const ustrRaw = '19300000000000000000'
    const wrongLeg = formatChartsPairTokenVolume(ustrRaw, 6)
    const rightLeg = formatChartsPairTokenVolume(ustrRaw, 18)
    expect(wrongLeg).not.toBe(rightLeg)
    expect(rightLeg).not.toMatch(/T$/)
  })

  it('6-dec 1000000 is the 1-token class', () => {
    expect(formatChartsPairTokenVolume('1000000', 6)).toBe(formatTokenAmount('1000000', 6))
    expect(formatChartsPairTokenVolume('1000000', 6)).toMatch(/^1/)
  })

  it('missing / non-integer / out-of-range decimals → em dash; never assume 6', () => {
    expect(formatChartsPairTokenVolume('1000000', undefined)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', null)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', Number.NaN)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', 6.5)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', -1)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', 19)).toBe('—')
    expect(formatChartsPairTokenVolume('1000000', 32767)).toBe('—')
  })

  it('invalid raw strings → em dash (no HTML inject, no tab lock)', () => {
    expect(formatChartsPairTokenVolume('', 6)).toBe('—')
    expect(formatChartsPairTokenVolume(null, 6)).toBe('—')
    expect(formatChartsPairTokenVolume('"><script>', 6)).toBe('—')
    expect(formatChartsPairTokenVolume('1.5', 6)).toBe('—')
    expect(formatChartsPairTokenVolume('9'.repeat(200), 6)).toBe('—')
  })
})

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

  it('reciprocals when inverted; missing stays em dash (GitLab #680)', () => {
    expect(formatTwapHumanPrice('206', 6, 6, true)).toMatch(/^0\.00485/)
    expect(formatTwapHumanPrice('1.05', 6, 6, true)).toMatch(/^0\.952/)
    expect(formatTwapHumanPrice('1.05', 6, 6, false)).toMatch(/^1\.05/)
    expect(formatTwapHumanPrice(null, 6, 6, true)).toBe('—')
    expect(formatTwapHumanPrice(Number.POSITIVE_INFINITY, 6, 6, true)).toBe('—')
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

  it('inverted 6/6 and 6/18 show reciprocal human, not USD (GitLab #680)', () => {
    const eq = formatTwapHumanPrice('1.05', 6, 6, true)
    expect(parseFloat(eq)).toBeCloseTo(1 / 1.05, 5)
    const mixed = formatTwapHumanPrice('111009000000000', 6, 18, true)
    expect(parseFloat(mixed)).toBeCloseTo(1 / 111.009, 4)
    expect(formatTwapHumanPrice(null, 6, 6, true)).toBe('—')
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
