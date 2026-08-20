import { describe, expect, it } from 'vitest'
import { formatProtocolCount, formatProtocolOracleUsd, formatProtocolPct, formatProtocolUsd } from '../formatProtocolStats'

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
  it('uses em-dash for non-finite census values', () => {
    expect(formatProtocolCount(undefined)).toBe('—')
    expect(formatProtocolCount(Number.NaN)).toBe('—')
  })
})

describe('formatProtocolOracleUsd', () => {
  it('still displays a depeg value instead of hardcoding $1', () => {
    expect(formatProtocolOracleUsd('0.87')).toContain('0.87')
    expect(formatProtocolOracleUsd('0.87')).not.toBe('$1.00')
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
