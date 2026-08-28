import { describe, expect, it } from 'vitest'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'
import { clampUst1DepositPrefillAmount, parseUst1AcquirePrefill } from '@/utils/ust1AcquirePrefill'
import type { Ust1EffectiveSwapView } from '@/utils/ust1WindowGates'

function params(record: Record<string, string>) {
  return { get: (key: string) => record[key] ?? null }
}

const nowSec = 1_700_000_000

function healthy(over: Partial<Ust1EffectiveSwapView> = {}): Ust1EffectiveSwapView {
  return {
    fee_bps: 100,
    per_tx_ust1_limit: '1000000000',
    rolling_24h_ust1_limit: '10000000000',
    paused: false,
    rolling_window_start_sec: nowSec - 100,
    rolling_volume_ust1: '0',
    max_oracle_age_sec: 21_600,
    oracle: {
      rate: UST1_RATE_SCALE.toString(),
      last_update_sec: nowSec - 30,
      paused: false,
    },
    ...over,
  }
}

describe('parseUst1AcquirePrefill (#678)', () => {
  it('reads deposit + positive human amount', () => {
    expect(parseUst1AcquirePrefill(params({ direction: 'deposit', amount: '1000' }))).toEqual({
      direction: 'deposit',
      amountHuman: '1000',
    })
  })

  it('ignores empty, junk, scientific, negative, and extra keys', () => {
    expect(parseUst1AcquirePrefill(params({}))).toEqual({ direction: null, amountHuman: null })
    expect(parseUst1AcquirePrefill(params({ amount: 'abc' })).amountHuman).toBeNull()
    expect(parseUst1AcquirePrefill(params({ amount: '1e99' })).amountHuman).toBeNull()
    expect(parseUst1AcquirePrefill(params({ amount: '-1' })).amountHuman).toBeNull()
    expect(parseUst1AcquirePrefill(params({ amount: '<img>' })).amountHuman).toBeNull()
    expect(parseUst1AcquirePrefill(params({ direction: 'mint', amount: '1', next: 'https://evil' }))).toEqual({
      direction: null,
      amountHuman: '1',
    })
  })
})

describe('clampUst1DepositPrefillAmount (#678)', () => {
  it('keeps a legal deposit', () => {
    expect(clampUst1DepositPrefillAmount('10', healthy(), nowSec)).toBe('10')
  })

  it('clamps huge amounts to remaining per-tx UST1', () => {
    const clamped = clampUst1DepositPrefillAmount('999999999999999', healthy(), nowSec)
    expect(clamped).not.toBeNull()
    expect(Number(clamped)).toBeLessThanOrEqual(1000 * 1.02)
  })

  it('fails closed when oracle is stale or paused', () => {
    expect(clampUst1DepositPrefillAmount('10', healthy({ paused: true }), nowSec)).toBeNull()
    expect(
      clampUst1DepositPrefillAmount(
        '10',
        healthy({ oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: 1, paused: false } }),
        nowSec
      )
    ).toBeNull()
    expect(
      clampUst1DepositPrefillAmount(
        '10',
        healthy({ oracle: { rate: '0', last_update_sec: nowSec - 30, paused: false } }),
        nowSec
      )
    ).toBeNull()
  })
})
