import { describe, it, expect } from 'vitest'
import { isNativeDenom, getWrappedEquivalent, getNativeEquivalent } from '../index'

describe('type helpers', () => {
  it('isNativeDenom identifies uluna and uusd', () => {
    expect(isNativeDenom('uluna')).toBe(true)
    expect(isNativeDenom('uusd')).toBe(true)
    expect(isNativeDenom('terra1abc')).toBe(false)
    expect(isNativeDenom('')).toBe(false)
  })

  it('getWrappedEquivalent returns null for unknown', () => {
    expect(getWrappedEquivalent('uatom')).toBeNull()
    expect(getWrappedEquivalent('terra1xyz')).toBeNull()
  })

  it('getNativeEquivalent returns null for unknown', () => {
    expect(getNativeEquivalent('terra1random')).toBeNull()
    expect(getNativeEquivalent('uluna')).toBeNull()
  })
})
