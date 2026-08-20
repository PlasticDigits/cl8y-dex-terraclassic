import { describe, expect, it } from 'vitest'
import {
  formatProtocolCount,
  formatProtocolFdusdOut,
  formatProtocolOracleUsd,
  formatProtocolUsd,
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
