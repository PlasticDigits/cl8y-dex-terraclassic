import { describe, it, expect } from 'vitest'
import { formatTokenAmount } from '../formatAmount'
import { formatChartsPairTokenVolume } from '../chartsPairStats'

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
