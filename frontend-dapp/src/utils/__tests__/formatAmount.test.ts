import { describe, it, expect } from 'vitest'
import { formatNum, formatTokenAmount, getDecimals } from '../formatAmount'

describe('formatNum', () => {
  it('returns 0 for zero', () => {
    expect(formatNum(0)).toBe('0')
  })

  it('returns 0 for NaN string', () => {
    expect(formatNum('abc')).toBe('0')
  })

  it('formats small numbers with sigfigs', () => {
    expect(formatNum(42.195, 3)).toBe('42.2')
  })

  it('formats thousands with K suffix', () => {
    expect(formatNum(13230, 4)).toBe('13.23K')
  })

  it('formats millions with M suffix', () => {
    expect(formatNum(1234567, 4)).toBe('1.235M')
  })

  it('formats billions with B suffix', () => {
    expect(formatNum(5_500_000_000, 3)).toBe('5.50B')
  })

  it('formats trillions with T suffix', () => {
    expect(formatNum(2_000_000_000_000, 3)).toBe('2.00T')
  })

  it('handles negative numbers', () => {
    expect(formatNum(-1500, 3)).toBe('-1.50K')
  })

  it('handles string input', () => {
    expect(formatNum('502498.5', 4)).toBe('502.5K')
  })

  it('handles empty string', () => {
    expect(formatNum('')).toBe('0')
  })
})

describe('formatTokenAmount', () => {
  it('returns 0 for empty string', () => {
    expect(formatTokenAmount('', 6)).toBe('0')
  })

  it('returns 0 for "0"', () => {
    expect(formatTokenAmount('0', 6)).toBe('0')
  })

  it('formats 1 token with 6 decimals', () => {
    expect(formatTokenAmount('1000000', 6)).toBe('1.000')
  })

  it('formats large amounts with abbreviation', () => {
    expect(formatTokenAmount('502498500503', 6)).toBe('502.5K')
  })

  it('formats with 18 decimals', () => {
    expect(formatTokenAmount('1000000000000000000', 18)).toBe('1.000')
  })

  it('returns 0 for invalid (non-numeric) input', () => {
    expect(formatTokenAmount('not_a_number', 6)).toBe('0')
  })

  it('returns 0 for decimal string input', () => {
    expect(formatTokenAmount('1.5', 6)).toBe('0')
  })

  it('handles very large amounts', () => {
    const result = formatTokenAmount('999999999999999999999999', 18)
    expect(result).toContain('M')
  })
})

describe('getDecimals', () => {
  it('returns 6 for unknown token', () => {
    const info = { token: { contract_addr: 'terra1unknown' } }
    expect(getDecimals(info)).toBe(6)
  })

  it('returns registry decimals for known CW20 token', () => {
    const info = { token: { contract_addr: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3' } }
    expect(getDecimals(info)).toBe(6)
  })

  it('returns registry decimals for known native denom', () => {
    const info = { native_token: { denom: 'uluna' } }
    expect(getDecimals(info)).toBe(6)
  })
})
