import { describe, it, expect, vi } from 'vitest'

const { MOCK_LUNC_C, MOCK_USTC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
}))

vi.mock('@/utils/constants', () => ({
  NATIVE_WRAPPED_PAIRS: {
    uluna: MOCK_LUNC_C,
    uusd: MOCK_USTC_C,
  } as Record<string, string>,
  WRAPPED_NATIVE_PAIRS: {
    [MOCK_LUNC_C]: 'uluna',
    [MOCK_USTC_C]: 'uusd',
  } as Record<string, string>,
}))

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

  it('getWrappedEquivalent returns LUNC-C for uluna', () => {
    expect(getWrappedEquivalent('uluna')).toBe(MOCK_LUNC_C)
  })

  it('getWrappedEquivalent returns USTC-C for uusd', () => {
    expect(getWrappedEquivalent('uusd')).toBe(MOCK_USTC_C)
  })

  it('getNativeEquivalent returns null for unknown', () => {
    expect(getNativeEquivalent('terra1random')).toBeNull()
    expect(getNativeEquivalent('uluna')).toBeNull()
  })

  it('getNativeEquivalent returns uluna for LUNC-C', () => {
    expect(getNativeEquivalent(MOCK_LUNC_C)).toBe('uluna')
  })

  it('getNativeEquivalent returns uusd for USTC-C', () => {
    expect(getNativeEquivalent(MOCK_USTC_C)).toBe('uusd')
  })
})
