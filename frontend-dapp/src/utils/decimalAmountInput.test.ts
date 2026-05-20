import { describe, expect, it } from 'vitest'
import { isDecimalAmountDraft, tryParseBigInt } from './decimalAmountInput'

describe('isDecimalAmountDraft', () => {
  it('accepts empty, integers, and a single decimal point', () => {
    expect(isDecimalAmountDraft('')).toBe(true)
    expect(isDecimalAmountDraft('0')).toBe(true)
    expect(isDecimalAmountDraft('42')).toBe(true)
    expect(isDecimalAmountDraft('0.01')).toBe(true)
    expect(isDecimalAmountDraft('.5')).toBe(true)
    expect(isDecimalAmountDraft('1.')).toBe(true)
  })

  it('rejects locale commas, letters, and other symbols', () => {
    expect(isDecimalAmountDraft(',')).toBe(false)
    expect(isDecimalAmountDraft('1,5')).toBe(false)
    expect(isDecimalAmountDraft('4^000000')).toBe(false)
    expect(isDecimalAmountDraft('4\\000000')).toBe(false)
    expect(isDecimalAmountDraft('abc')).toBe(false)
    expect(isDecimalAmountDraft('1.2.3')).toBe(false)
  })
})

describe('tryParseBigInt', () => {
  it('parses digit-only raw strings', () => {
    expect(tryParseBigInt('0')).toBe(0n)
    expect(tryParseBigInt('1500000')).toBe(1500000n)
  })

  it('returns null for invalid raw strings', () => {
    expect(tryParseBigInt('')).toBe(null)
    expect(tryParseBigInt('4^0')).toBe(null)
    expect(tryParseBigInt('1.5')).toBe(null)
  })
})
