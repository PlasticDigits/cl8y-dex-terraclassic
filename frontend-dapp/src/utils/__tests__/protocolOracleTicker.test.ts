import { describe, it, expect } from 'vitest'
import { parseProtocolOracleTicker, PROTOCOL_ORACLE_TICKERS } from '../protocolOracleTicker'
import { formatProtocolUsd, formatProtocolCount, formatProtocolOracleUsd } from '../formatProtocolStats'

describe('parseProtocolOracleTicker', () => {
  it('allowlists ustc, lunc, vfdusd', () => {
    expect(parseProtocolOracleTicker('ustc')).toBe('ustc')
    expect(parseProtocolOracleTicker('LUNC')).toBe('lunc')
    expect(parseProtocolOracleTicker('vfdusd')).toBe('vfdusd')
    expect(PROTOCOL_ORACLE_TICKERS).toEqual(['ustc', 'lunc', 'vfdusd'])
  })

  it('defaults unknown, HTML, javascript, and path injection to ustc', () => {
    expect(parseProtocolOracleTicker('btc')).toBe('ustc')
    expect(parseProtocolOracleTicker('../ustc')).toBe('ustc')
    expect(parseProtocolOracleTicker('javascript:alert(1)')).toBe('ustc')
    expect(parseProtocolOracleTicker('<img src=x onerror=alert(1)>')).toBe('ustc')
    expect(parseProtocolOracleTicker('fdusd')).toBe('ustc')
    expect(parseProtocolOracleTicker(null)).toBe('ustc')
    expect(parseProtocolOracleTicker('ustc ')).toBe('ustc')
  })
})

describe('formatProtocolUsd', () => {
  it('formats finite USD and em-dashes non-finite', () => {
    expect(formatProtocolUsd('12.5')).toMatch(/^\$/)
    expect(formatProtocolUsd('Infinity')).toBe('—')
    expect(formatProtocolUsd(Number.NaN)).toBe('—')
    expect(formatProtocolUsd(undefined)).toBe('—')
    expect(formatProtocolCount(undefined)).toBe('—')
    expect(formatProtocolOracleUsd('0.87')).toContain('0.87')
  })
})
