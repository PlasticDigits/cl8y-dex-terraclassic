import { describe, it, expect, vi } from 'vitest'

const { MOCK_LUNC_C, MOCK_USTC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>()
  return {
    ...actual,
    WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1wrap_mapper_mock',
    TREASURY_CONTRACT_ADDRESS: 'terra1treasury_mock',
    LUNC_C_TOKEN_ADDRESS: MOCK_LUNC_C,
    USTC_C_TOKEN_ADDRESS: MOCK_USTC_C,
    NATIVE_WRAPPED_PAIRS: {
      uluna: MOCK_LUNC_C,
      uusd: MOCK_USTC_C,
    } as Record<string, string>,
    WRAPPED_NATIVE_PAIRS: {
      [MOCK_LUNC_C]: 'uluna',
      [MOCK_USTC_C]: 'uusd',
    } as Record<string, string>,
  }
})

import {
  isNativeWrappedPair,
  getWrappedForNative,
  getNativeForWrapped,
  isNativeToken,
  isWrappedNative,
} from '../wrapMapper'

describe('wrapMapper helpers', () => {
  it('isNativeToken identifies native denoms', () => {
    expect(isNativeToken('uluna')).toBe(true)
    expect(isNativeToken('uusd')).toBe(true)
    expect(isNativeToken('terra1abc')).toBe(false)
  })

  it('getWrappedForNative returns null for unknown denoms', () => {
    expect(getWrappedForNative('uatom')).toBeNull()
  })

  it('getWrappedForNative returns LUNC-C for uluna', () => {
    expect(getWrappedForNative('uluna')).toBe(MOCK_LUNC_C)
  })

  it('getWrappedForNative returns USTC-C for uusd', () => {
    expect(getWrappedForNative('uusd')).toBe(MOCK_USTC_C)
  })

  it('getNativeForWrapped returns null for unknown tokens', () => {
    expect(getNativeForWrapped('terra1unknown')).toBeNull()
  })

  it('getNativeForWrapped returns uluna for LUNC-C', () => {
    expect(getNativeForWrapped(MOCK_LUNC_C)).toBe('uluna')
  })

  it('getNativeForWrapped returns uusd for USTC-C', () => {
    expect(getNativeForWrapped(MOCK_USTC_C)).toBe('uusd')
  })

  it('isNativeWrappedPair returns false for unrelated tokens', () => {
    expect(isNativeWrappedPair('terra1abc', 'terra1def')).toBe(false)
  })

  it('isNativeWrappedPair returns true for uluna/LUNC-C', () => {
    expect(isNativeWrappedPair('uluna', MOCK_LUNC_C)).toBe(true)
  })

  it('isNativeWrappedPair returns true for LUNC-C/uluna (reverse order)', () => {
    expect(isNativeWrappedPair(MOCK_LUNC_C, 'uluna')).toBe(true)
  })

  it('isNativeWrappedPair returns true for uusd/USTC-C', () => {
    expect(isNativeWrappedPair('uusd', MOCK_USTC_C)).toBe(true)
  })

  it('isWrappedNative identifies wrapped native tokens', () => {
    expect(isWrappedNative(MOCK_LUNC_C)).toBe(true)
    expect(isWrappedNative(MOCK_USTC_C)).toBe(true)
    expect(isWrappedNative('terra1random')).toBe(false)
  })
})
