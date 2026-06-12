import { describe, expect, it } from 'vitest'
import { isDecimalAmountDraft, isPositiveDecimalAmount, tryParseBigInt } from './decimalAmountInput'

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

describe('isPositiveDecimalAmount', () => {
  const aboveSafeInt = '9007199254740992'

  it('is false for empty, zero, and invalid drafts', () => {
    expect(isPositiveDecimalAmount('')).toBe(false)
    expect(isPositiveDecimalAmount('0')).toBe(false)
    expect(isPositiveDecimalAmount('0.0')).toBe(false)
    expect(isPositiveDecimalAmount('1,5')).toBe(false)
  })

  it('is true for positive drafts including above Number.MAX_SAFE_INTEGER', () => {
    expect(isPositiveDecimalAmount('0.01')).toBe(true)
    expect(isPositiveDecimalAmount(aboveSafeInt)).toBe(true)
    expect(isPositiveDecimalAmount(`${aboveSafeInt}.5`)).toBe(true)
  })

  it('does not rely on parseFloat for integers above MAX_SAFE_INTEGER', () => {
    expect(parseFloat('9007199254740993')).toBe(9007199254740992)
    expect(isPositiveDecimalAmount('9007199254740993')).toBe(true)
    expect(isPositiveDecimalAmount(`${aboveSafeInt}1`)).toBe(true)
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
