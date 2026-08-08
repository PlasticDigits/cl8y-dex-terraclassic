import { describe, it, expect, vi, beforeEach } from 'vitest'

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

const queryContract = vi.fn()
vi.mock('../queries', () => ({
  queryContract: (...args: unknown[]) => queryContract(...args),
}))

import {
  isNativeWrappedPair,
  getWrappedForNative,
  getNativeForWrapped,
  isNativeToken,
  isWrappedNative,
  netAfterWrapMapperFee,
  amountForTargetNetAfterWrapMapperFee,
  wrapUnwrapFeeNote,
  queryWrapMapperFeeBps,
  clearWrapMapperConfigCache,
} from '../wrapMapper'

describe('wrapMapper helpers', () => {
  beforeEach(() => {
    clearWrapMapperConfigCache()
    queryContract.mockReset()
  })

  it('isNativeToken identifies native denoms', () => {
    expect(isNativeToken('uluna')).toBe(true)
    expect(isNativeToken('uusd')).toBe(true)
    expect(isNativeToken('terra1abc')).toBe(false)
  })

  it('getWrappedForNative returns null for unknown denoms', () => {
    expect(getWrappedForNative('uatom')).toBeNull()
  })

  it('getWrappedForNative returns cLUNC address for uluna', () => {
    expect(getWrappedForNative('uluna')).toBe(MOCK_LUNC_C)
  })

  it('getWrappedForNative returns cUSTC address for uusd', () => {
    expect(getWrappedForNative('uusd')).toBe(MOCK_USTC_C)
  })

  it('getNativeForWrapped returns null for unknown tokens', () => {
    expect(getNativeForWrapped('terra1unknown')).toBeNull()
  })

  it('getNativeForWrapped returns uluna for cLUNC', () => {
    expect(getNativeForWrapped(MOCK_LUNC_C)).toBe('uluna')
  })

  it('getNativeForWrapped returns uusd for cUSTC', () => {
    expect(getNativeForWrapped(MOCK_USTC_C)).toBe('uusd')
  })

  it('isNativeWrappedPair returns false for unrelated tokens', () => {
    expect(isNativeWrappedPair('terra1abc', 'terra1def')).toBe(false)
  })

  it('isNativeWrappedPair returns true for uluna/cLUNC', () => {
    expect(isNativeWrappedPair('uluna', MOCK_LUNC_C)).toBe(true)
  })

  it('isNativeWrappedPair returns true for cLUNC/uluna (reverse order)', () => {
    expect(isNativeWrappedPair(MOCK_LUNC_C, 'uluna')).toBe(true)
  })

  it('isNativeWrappedPair returns true for uusd/cUSTC', () => {
    expect(isNativeWrappedPair('uusd', MOCK_USTC_C)).toBe(true)
  })

  it('isWrappedNative identifies wrapped native tokens', () => {
    expect(isWrappedNative(MOCK_LUNC_C)).toBe(true)
    expect(isWrappedNative(MOCK_USTC_C)).toBe(true)
    expect(isWrappedNative('terra1random')).toBe(false)
  })
})

describe('wrap-mapper fee math (GitLab #507)', () => {
  it('netAfterWrapMapperFee skims floor(amount * bps / 10000)', () => {
    expect(netAfterWrapMapperFee(1_000_000n, 100)).toBe(990_000n)
    expect(netAfterWrapMapperFee(1_000_000n, 50)).toBe(995_000n)
    expect(netAfterWrapMapperFee(1_000_000n, 0)).toBe(1_000_000n)
    expect(netAfterWrapMapperFee(2_000_000n, 1)).toBe(1_999_800n)
  })

  it('amountForTargetNetAfterWrapMapperFee inverts fee skim', () => {
    const target = 990_000n
    const gross = amountForTargetNetAfterWrapMapperFee(target, 100)
    expect(netAfterWrapMapperFee(gross, 100)).toBeGreaterThanOrEqual(target)
    expect(gross).toBe(1_000_000n)
  })

  it('wrapUnwrapFeeNote avoids false 1:1 when fee applies', () => {
    expect(wrapUnwrapFeeNote('wrap', 0)).toBe('Wrap (1:1)')
    expect(wrapUnwrapFeeNote('unwrap', 0)).toBe('Unwrap (1:1)')
    expect(wrapUnwrapFeeNote('wrap', 100)).toBe('Wrap (1.00% fee)')
    expect(wrapUnwrapFeeNote('unwrap', 50)).toBe('Unwrap (0.50% fee)')
  })

  it('queryWrapMapperFeeBps reads on-chain config', async () => {
    queryContract.mockResolvedValue({
      governance: 'terra1gov',
      treasury: 'terra1treasury',
      paused: false,
      fee_bps: 100,
    })
    await expect(queryWrapMapperFeeBps()).resolves.toBe(100)
  })
})
