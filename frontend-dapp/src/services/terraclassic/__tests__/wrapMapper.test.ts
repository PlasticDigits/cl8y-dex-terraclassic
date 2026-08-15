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
  queryWrapMapperConfig,
  queryPausedState,
  checkRateLimitExceeded,
  wrapTreasuryMatchesEnv,
  clearWrapMapperConfigCache,
  parseWrapMapperFeePair,
  wrapMapperFeeBps,
  retuneUnwrapFeeBps,
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
  beforeEach(() => {
    clearWrapMapperConfigCache()
    queryContract.mockReset()
  })

  it('netAfterWrapMapperFee skims floor(amount * bps / 10000)', () => {
    expect(netAfterWrapMapperFee(1_000_000n, 100)).toBe(990_000n)
    expect(netAfterWrapMapperFee(1_000_000n, 50)).toBe(995_000n)
    expect(netAfterWrapMapperFee(1_000_000n, 0)).toBe(1_000_000n)
    expect(netAfterWrapMapperFee(2_000_000n, 1)).toBe(1_999_800n)
    expect(netAfterWrapMapperFee(1n, 100)).toBe(1n)
  })

  it('amountForTargetNetAfterWrapMapperFee inverts fee skim', () => {
    const target = 990_000n
    const gross = amountForTargetNetAfterWrapMapperFee(target, 100)
    expect(netAfterWrapMapperFee(gross, 100)).toBeGreaterThanOrEqual(target)
    expect(gross).toBe(1_000_000n)
  })

  it('amountForTargetNetAfterWrapMapperFee returns 0 when fee_bps >= 10000', () => {
    expect(amountForTargetNetAfterWrapMapperFee(990_000n, 10_000)).toBe(0n)
  })

  it('wrapUnwrapFeeNote avoids false 1:1 when fee applies or is unknown', () => {
    expect(wrapUnwrapFeeNote('wrap', 0)).toBe('Wrap (1:1)')
    expect(wrapUnwrapFeeNote('unwrap', 0)).toBe('Unwrap (1:1)')
    expect(wrapUnwrapFeeNote('wrap', 100)).toBe('Wrap (1.00% fee)')
    expect(wrapUnwrapFeeNote('unwrap', 50)).toBe('Unwrap (0.50% fee; You Receive after burn tax)')
    expect(wrapUnwrapFeeNote('unwrap', 200, '0.015')).toBe('Unwrap (2.00% fee; You Receive after 1.5% burn tax)')
    expect(wrapUnwrapFeeNote('unwrap', 51, '0.015')).toBe('Unwrap (0.51% fee; You Receive after 1.5% burn tax)')
    expect(wrapUnwrapFeeNote('wrap', null)).toBe('Wrap fee unavailable')
    expect(wrapUnwrapFeeNote('unwrap', undefined)).toBe('Unwrap fee unavailable')
  })

  it('parseWrapMapperFeePair reads split fees and fails closed on partial', () => {
    expect(parseWrapMapperFeePair({ fee_wrap_bps: 200, fee_unwrap_bps: 51 })).toEqual({
      fee_wrap_bps: 200,
      fee_unwrap_bps: 51,
    })
    expect(parseWrapMapperFeePair({ fee_wrap_bps: '200', fee_unwrap_bps: '51' })).toEqual({
      fee_wrap_bps: 200,
      fee_unwrap_bps: 51,
    })
    expect(parseWrapMapperFeePair({ fee_bps: 200 })).toEqual({
      fee_wrap_bps: 200,
      fee_unwrap_bps: 200,
    })
    expect(parseWrapMapperFeePair({ fee_wrap_bps: 200 })).toBeNull()
    expect(parseWrapMapperFeePair({ fee_unwrap_bps: 51 })).toBeNull()
    expect(parseWrapMapperFeePair({ fee_wrap_bps: 200, fee_unwrap_bps: -1 })).toBeNull()
    expect(parseWrapMapperFeePair({})).toBeNull()
    expect(parseWrapMapperFeePair(null)).toBeNull()
  })

  it('wrapMapperFeeBps picks wrap vs unwrap and does not swap fields', () => {
    const split = { fee_wrap_bps: 200, fee_unwrap_bps: 51 }
    expect(wrapMapperFeeBps(split, 'wrap')).toBe(200)
    expect(wrapMapperFeeBps(split, 'unwrap')).toBe(51)
    expect(wrapMapperFeeBps({ fee_bps: 200 }, 'wrap')).toBe(200)
    expect(wrapMapperFeeBps({ fee_bps: 200 }, 'unwrap')).toBe(200)
    expect(wrapMapperFeeBps({ fee_wrap_bps: 200 }, 'unwrap')).toBeNull()
  })

  it('retuneUnwrapFeeBps matches 1.5% tax → 51 (ustr-cmm#9)', () => {
    expect(retuneUnwrapFeeBps(0.015)).toBe(51)
    expect(() => retuneUnwrapFeeBps(0.02)).toThrow(/subsidy/i)
  })

  it('queryWrapMapperFeeBps reads split on-chain config (#516)', async () => {
    queryContract.mockResolvedValue({
      governance: 'terra1gov',
      treasury: 'terra1treasury_mock',
      paused: false,
      fee_wrap_bps: 200,
      fee_unwrap_bps: 51,
    })
    await expect(queryWrapMapperFeeBps('wrap')).resolves.toBe(200)
    await expect(queryWrapMapperFeeBps('unwrap')).resolves.toBe(51)
  })

  it('queryWrapMapperFeeBps maps transitional fee_bps to both sides', async () => {
    queryContract.mockResolvedValue({
      governance: 'terra1gov',
      treasury: 'terra1treasury_mock',
      paused: false,
      fee_bps: 100,
    })
    await expect(queryWrapMapperFeeBps('wrap')).resolves.toBe(100)
    await expect(queryWrapMapperFeeBps('unwrap')).resolves.toBe(100)
  })

  it('queryWrapMapperConfig returns null on partial split fees (fail closed)', async () => {
    queryContract.mockResolvedValue({
      governance: 'terra1gov',
      treasury: 'terra1treasury_mock',
      paused: false,
      fee_wrap_bps: 200,
    })
    await expect(queryWrapMapperConfig()).resolves.toBeNull()
    await expect(queryWrapMapperFeeBps('unwrap')).rejects.toThrow(/config unavailable/i)
  })

  it('queryWrapMapperFeeBps throws when config unavailable (fail closed)', async () => {
    queryContract.mockRejectedValue(new Error('LCD down'))
    await expect(queryWrapMapperFeeBps()).rejects.toThrow(/config unavailable/i)
  })

  it('queryPausedState returns null when config unavailable', async () => {
    queryContract.mockRejectedValue(new Error('LCD down'))
    await expect(queryPausedState()).resolves.toBeNull()
  })

  it('checkRateLimitExceeded returns null when LCD fails', async () => {
    queryContract.mockRejectedValue(new Error('LCD down'))
    await expect(checkRateLimitExceeded('uluna', '1000')).resolves.toBeNull()
  })

  it('checkRateLimitExceeded treats expired window as full capacity', async () => {
    const startSec = Math.floor(Date.now() / 1000) - 10_000
    queryContract.mockResolvedValue({
      config: { max_amount_per_window: '1000', window_seconds: 3600 },
      current_window_start: String(startSec),
      amount_used: '1000',
    })
    await expect(checkRateLimitExceeded('uluna', '500')).resolves.toBe(false)
  })

  it('checkRateLimitExceeded is true when amount exceeds remaining', async () => {
    const startSec = Math.floor(Date.now() / 1000) - 10
    queryContract.mockResolvedValue({
      config: { max_amount_per_window: '1000', window_seconds: 3600 },
      current_window_start: String(startSec),
      amount_used: '900',
    })
    await expect(checkRateLimitExceeded('uluna', '200')).resolves.toBe(true)
    await expect(checkRateLimitExceeded('uluna', '100')).resolves.toBe(false)
  })

  it('wrapTreasuryMatchesEnv requires exact treasury match', () => {
    expect(
      wrapTreasuryMatchesEnv({
        governance: 'terra1gov',
        treasury: 'terra1treasury_mock',
        paused: false,
        fee_wrap_bps: 200,
        fee_unwrap_bps: 51,
      })
    ).toBe(true)
    expect(
      wrapTreasuryMatchesEnv({
        governance: 'terra1gov',
        treasury: 'terra1other',
        paused: false,
        fee_wrap_bps: 200,
        fee_unwrap_bps: 51,
      })
    ).toBe(false)
  })
})
