import { describe, expect, it } from 'vitest'
import {
  evaluateUst1SubmitGate,
  isOracleStale,
  rollingRemainingUst1,
  type Ust1EffectiveSwapView,
} from '@/utils/ust1WindowGates'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'

const NOW = 1_700_000_000

function healthyView(over: Partial<Ust1EffectiveSwapView> = {}): Ust1EffectiveSwapView {
  const baseOracle = {
    rate: UST1_RATE_SCALE.toString(),
    last_update_sec: NOW - 60,
    paused: false,
  }
  const { oracle: overOracle, ...rest } = over
  return {
    fee_bps: 100,
    per_tx_ust1_limit: '1000000000',
    rolling_24h_ust1_limit: '10000000000',
    paused: false,
    rolling_window_start_sec: NOW - 3600,
    rolling_volume_ust1: '1000000000',
    max_oracle_age_sec: 21_600,
    ...rest,
    oracle: { ...baseOracle, ...(overOracle ?? {}) },
  }
}

const baseOk = {
  windowEnabled: true,
  walletConnected: true,
  direction: 'deposit' as const,
  amountRaw: 1_000_000n,
  amountDraftEmpty: false,
  balanceRaw: 10_000_000n,
  viewLoading: false,
  viewError: false,
  submitting: false,
  nowSec: NOW,
}

describe('ust1WindowGates (#506)', () => {
  it('computes rolling remaining and resets after 24h', () => {
    const view = healthyView()
    expect(rollingRemainingUst1(view, NOW)).toBe(9_000_000_000n)
    expect(rollingRemainingUst1(view, NOW + 86_400)).toBe(10_000_000_000n)
  })

  it('detects stale oracle and last_update_sec=0', () => {
    expect(isOracleStale(healthyView(), NOW)).toBe(false)
    expect(
      isOracleStale(healthyView({ oracle: { rate: '1', last_update_sec: NOW - 30_000, paused: false } }), NOW)
    ).toBe(true)
    expect(isOracleStale(healthyView({ oracle: { rate: '1', last_update_sec: 0, paused: false } }), NOW)).toBe(true)
  })

  it('allows healthy deposit quote', () => {
    const r = evaluateUst1SubmitGate({ ...baseOk, view: healthyView() })
    expect(r.canSubmit).toBe(true)
    expect(r.receiveRaw).toBe(990_000n)
    expect(r.ctaLabel).toBe('Deposit')
  })

  it('blocks pause / stale / limits / wrong balance', () => {
    expect(evaluateUst1SubmitGate({ ...baseOk, view: healthyView({ paused: true }) }).reason).toBe('window_paused')
    expect(
      evaluateUst1SubmitGate({
        ...baseOk,
        view: healthyView({ oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: NOW - 60, paused: true } }),
      }).reason
    ).toBe('oracle_paused')
    expect(
      evaluateUst1SubmitGate({
        ...baseOk,
        view: healthyView({
          oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: NOW - 30_000, paused: false },
        }),
      }).reason
    ).toBe('oracle_stale')
    expect(
      evaluateUst1SubmitGate({
        ...baseOk,
        amountRaw: 2_000_000_000n,
        balanceRaw: 3_000_000_000n,
        view: healthyView(),
      }).reason
    ).toBe('per_tx_limit')
    expect(
      evaluateUst1SubmitGate({
        ...baseOk,
        amountRaw: 600_000_000n,
        balanceRaw: 1_000_000_000n,
        view: healthyView({ rolling_volume_ust1: '9500000000' }),
      }).reason
    ).toBe('rolling_limit')
    expect(
      evaluateUst1SubmitGate({
        ...baseOk,
        amountRaw: 5_000_000n,
        balanceRaw: 1_000_000n,
        view: healthyView(),
      }).reason
    ).toBe('insufficient_balance')
  })

  it('hides page safely when window env missing', () => {
    const r = evaluateUst1SubmitGate({
      ...baseOk,
      windowEnabled: false,
      view: undefined,
    })
    expect(r.reason).toBe('window_unavailable')
    expect(r.canSubmit).toBe(false)
  })

  it('withdraw uses gross UST1 as notional', () => {
    const r = evaluateUst1SubmitGate({
      ...baseOk,
      direction: 'withdraw',
      amountRaw: 1_000_000n,
      view: healthyView(),
    })
    expect(r.canSubmit).toBe(true)
    expect(r.ust1Notional).toBe(1_000_000n)
    expect(r.receiveRaw).toBe(990_000n)
    expect(r.ctaLabel).toBe('Withdraw')
  })
})
