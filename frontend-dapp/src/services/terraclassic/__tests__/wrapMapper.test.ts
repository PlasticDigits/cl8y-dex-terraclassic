import { describe, it, expect } from 'vitest'
import { isNativeWrappedPair, getWrappedForNative, getNativeForWrapped, isNativeToken } from '../wrapMapper'

describe('wrapMapper helpers', () => {
  it('isNativeToken identifies native denoms', () => {
    expect(isNativeToken('uluna')).toBe(true)
    expect(isNativeToken('uusd')).toBe(true)
    expect(isNativeToken('terra1abc')).toBe(false)
  })

  it('getWrappedForNative returns null for unknown denoms', () => {
    expect(getWrappedForNative('uatom')).toBeNull()
  })

  it('getNativeForWrapped returns null for unknown tokens', () => {
    expect(getNativeForWrapped('terra1unknown')).toBeNull()
  })

  it('isNativeWrappedPair returns false for unrelated tokens', () => {
    expect(isNativeWrappedPair('terra1abc', 'terra1def')).toBe(false)
  })
})
