import { describe, expect, it } from 'vitest'
import {
  formatProtocolCount,
  formatProtocolFdusdOut,
  formatProtocolOracleUsd,
  formatProtocolPct,
  formatProtocolUsd,
  protocolPctToneFromDisplay,
} from '../formatProtocolStats'

describe('formatProtocolUsd', () => {
  it('formats finite USD with a dollar prefix', () => {
    expect(formatProtocolUsd('1234.5')).toMatch(/^\$/)
    expect(formatProtocolUsd(0)).toMatch(/^\$/)
  })

  it('uses em-dash for missing or non-finite values', () => {
    expect(formatProtocolUsd(undefined)).toBe('—')
    expect(formatProtocolUsd(null)).toBe('—')
    expect(formatProtocolUsd('')).toBe('—')
    expect(formatProtocolUsd(Number.POSITIVE_INFINITY)).toBe('—')
    expect(formatProtocolUsd(Number.NaN)).toBe('—')
  })
})

describe('formatProtocolCount', () => {
  it('renders small integers without trailing decimals (GitLab #667)', () => {
    expect(formatProtocolCount(0)).toBe('0')
    expect(formatProtocolCount(8)).toBe('8')
    expect(formatProtocolCount(14)).toBe('14')
    expect(formatProtocolCount(151)).toBe('151')
    expect(formatProtocolCount('14.00')).toBe('14')
    expect(formatProtocolCount(14)).not.toMatch(/\.0/)
  })

  it('uses compact K only when the count is actually ≥ 1000', () => {
    expect(formatProtocolCount(999)).toBe('999')
    expect(formatProtocolCount(1000)).toMatch(/K$/)
    expect(formatProtocolCount(1000)).not.toMatch(/1000\.00|1,000\.00/)
  })

  it('uses em-dash for non-finite, negative, or XSS-like census values', () => {
    expect(formatProtocolCount(undefined)).toBe('—')
    expect(formatProtocolCount(null)).toBe('—')
    expect(formatProtocolCount('')).toBe('—')
    expect(formatProtocolCount(Number.NaN)).toBe('—')
    expect(formatProtocolCount(Number.POSITIVE_INFINITY)).toBe('—')
    expect(formatProtocolCount(-1)).toBe('—')
    expect(formatProtocolCount('<script>')).toBe('—')
    expect(formatProtocolCount('javascript:alert(1)')).toBe('—')
  })
})

describe('formatProtocolOracleUsd', () => {
  it('still displays a depeg value instead of hardcoding $1', () => {
    expect(formatProtocolOracleUsd('0.87')).toContain('0.87')
    expect(formatProtocolOracleUsd('0.87')).not.toBe('$1.00')
  })
})

describe('formatProtocolFdusdOut', () => {
  it('formats a known Venus fixture without a dollar prefix', () => {
    expect(formatProtocolFdusdOut('0.023')).toContain('0.023')
    expect(formatProtocolFdusdOut('0.023')).not.toMatch(/^\$/)
  })

  it('uses em-dash for missing, non-finite, zero, or overflow — never a fake 1.0', () => {
    expect(formatProtocolFdusdOut(undefined)).toBe('—')
    expect(formatProtocolFdusdOut(null)).toBe('—')
    expect(formatProtocolFdusdOut('')).toBe('—')
    expect(formatProtocolFdusdOut(0)).toBe('—')
    expect(formatProtocolFdusdOut(Number.POSITIVE_INFINITY)).toBe('—')
    expect(formatProtocolFdusdOut(Number.NaN)).toBe('—')
    expect(formatProtocolFdusdOut('1e309')).toBe('—')
  })
})

describe('formatProtocolPct', () => {
  it('renders signed compact percents', () => {
    expect(formatProtocolPct('12.5')).toMatch(/^\+.*%$/)
    expect(formatProtocolPct('-3')).toMatch(/^-.*%$/)
    expect(formatProtocolPct(0)).toBe('0%')
  })

  it('uses em-dash for missing, non-finite, or XSS-like strings', () => {
    expect(formatProtocolPct(undefined)).toBe('—')
    expect(formatProtocolPct(null)).toBe('—')
    expect(formatProtocolPct('')).toBe('—')
    expect(formatProtocolPct(Number.POSITIVE_INFINITY)).toBe('—')
    expect(formatProtocolPct(Number.NEGATIVE_INFINITY)).toBe('—')
    expect(formatProtocolPct(Number.NaN)).toBe('—')
    expect(formatProtocolPct('<script>')).toBe('—')
    expect(formatProtocolPct('javascript:alert(1)')).toBe('—')
  })
})

describe('protocolPctToneFromDisplay', () => {
  it('uses semantic tokens, never gold as a fill', () => {
    expect(protocolPctToneFromDisplay('+12%')).toBe('var(--color-positive)')
    expect(protocolPctToneFromDisplay('-3%')).toBe('var(--color-negative)')
    expect(protocolPctToneFromDisplay('0%')).toBe('var(--ink-dim)')
    expect(protocolPctToneFromDisplay('—')).toBe('var(--ink-dim)')
  })
})
