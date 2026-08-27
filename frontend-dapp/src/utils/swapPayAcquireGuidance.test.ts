import { describe, expect, it } from 'vitest'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'
import { assertSecondaryMarketCopy } from '@/utils/ust1SecondaryMarket'
import type { Ust1EffectiveSwapView } from '@/utils/ust1WindowGates'
import {
  SWAP_ACQUIRE_COPY,
  SWAP_ACQUIRE_GUIDE_UST1_PATH,
  SWAP_ACQUIRE_GUIDE_WRAP_PATH,
  buildUst1DepositHref,
  evaluateSwapPayAcquireGuidance,
  isAllowedAcquireHref,
  isUst1PayAsset,
  type EvaluateSwapPayAcquireInput,
} from '@/utils/swapPayAcquireGuidance'

const UST1 = 'terra1ust100000000000000000000000000000000001'
const CLUNC = 'terra1clunc0000000000000000000000000000000001'
const OTHER = 'terra1other000000000000000000000000000000001'
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

function base(over: Partial<EvaluateSwapPayAcquireInput> = {}): EvaluateSwapPayAcquireInput {
  return {
    walletConnected: true,
    hasPositivePay: true,
    hasSettledQuote: true,
    payAsset: OTHER,
    paySymbol: 'GEM',
    payDecimals: 6,
    payRaw: 1_000_000n,
    payBalanceRaw: 1_000_000n,
    vfdusdBalanceRaw: null,
    ust1TokenAddress: UST1,
    windowEnabled: true,
    windowView: healthy(),
    windowViewError: false,
    wrapEnabled: true,
    wrappedPayAssets: new Set([CLUNC]),
    expectedSlippagePct: 1,
    nowSec,
    ...over,
  }
}

describe('evaluateSwapPayAcquireGuidance (#678)', () => {
  it('is ok when pay is empty or funded with low impact', () => {
    expect(evaluateSwapPayAcquireGuidance(base({ hasPositivePay: false })).kind).toBe('ok')
    expect(evaluateSwapPayAcquireGuidance(base()).kind).toBe('ok')
  })

  it('marks disconnected quote without inventing a balance', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({ walletConnected: false, payBalanceRaw: null, expectedSlippagePct: 12 })
    )
    expect(g.kind).toBe('disconnected_quote')
    expect(g.message).toBeNull()
    expect(g.guideHref).toBeNull()
  })

  it('UST1 shortfall under cap suggests inverse + capped /ust1 href', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({
        payAsset: UST1,
        paySymbol: 'UST1',
        payRaw: 500_000_000n,
        payBalanceRaw: 0n,
      })
    )
    expect(g.kind).toBe('insufficient_ust1_window')
    expect(g.guideHref).toMatch(/\/ust1\?direction=deposit&amount=/)
    expect(g.guideLabel).toBe('Get UST1')
    expect(g.suggestedVfdusdHuman).toBeTruthy()
    expect(g.message).toMatch(/Deposit about/)
    expect(g.message).not.toMatch(/49,?999/)
  })

  it('UST1 shortfall over per-tx does not promise a failing deposit', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({
        payAsset: UST1,
        paySymbol: 'UST1',
        payRaw: 50_000_000_000n,
        payBalanceRaw: 0n,
      })
    )
    expect(g.kind).toBe('insufficient_ust1_over_window')
    expect(g.message).toBe(SWAP_ACQUIRE_COPY.insufficientUst1OverWindow)
    expect(g.guideHref?.startsWith(SWAP_ACQUIRE_GUIDE_UST1_PATH)).toBe(true)
    expect(g.suggestedVfdusdHuman).not.toBe('49999')
    if (g.suggestedVfdusdHuman) {
      expect(Number(g.suggestedVfdusdHuman)).toBeLessThanOrEqual(1000 * 1.02)
    }
  })

  it('UST1 shortfall over rolling remaining is over-window', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({
        payAsset: UST1,
        paySymbol: 'UST1',
        payRaw: 2_000_000_000n,
        payBalanceRaw: 0n,
        windowView: healthy({ rolling_24h_ust1_limit: '1500000000', rolling_volume_ust1: '1000000000' }),
      })
    )
    expect(g.kind).toBe('insufficient_ust1_over_window')
  })

  it('window null / error / stale / paused fail closed to generic (no invented vFDUSD)', () => {
    for (const over of [
      { windowEnabled: false, windowView: null },
      { windowView: null, windowViewError: true },
      { windowView: healthy({ paused: true }) },
      {
        windowView: healthy({
          oracle: { rate: UST1_RATE_SCALE.toString(), last_update_sec: 1, paused: false },
        }),
      },
    ] as Partial<EvaluateSwapPayAcquireInput>[]) {
      const g = evaluateSwapPayAcquireGuidance(
        base({ payAsset: UST1, paySymbol: 'UST1', payRaw: 2_000_000n, payBalanceRaw: 0n, ...over })
      )
      expect(g.kind).toBe('insufficient_generic')
      expect(g.suggestedVfdusdHuman).toBeNull()
      expect(g.message).toBe(SWAP_ACQUIRE_COPY.insufficientGeneric('UST1'))
    }
  })

  it('wrap-native shortfall guides to /wrap', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({ payAsset: CLUNC, paySymbol: 'cLUNC', payRaw: 2_000_000n, payBalanceRaw: 0n })
    )
    expect(g.kind).toBe('insufficient_wrap')
    expect(g.guideHref).toBe(SWAP_ACQUIRE_GUIDE_WRAP_PATH)
  })

  it('generic CW20 shortfall has no Guide', () => {
    const g = evaluateSwapPayAcquireGuidance(base({ payRaw: 2_000_000n, payBalanceRaw: 0n }))
    expect(g.kind).toBe('insufficient_generic')
    expect(g.guideHref).toBeNull()
  })

  it('unfunded high impact prefers insufficient over high_impact', () => {
    const g = evaluateSwapPayAcquireGuidance(base({ payRaw: 2_000_000n, payBalanceRaw: 0n, expectedSlippagePct: 12 }))
    expect(g.kind).toBe('insufficient_generic')
  })

  it('funded high impact offers reduce never above balance', () => {
    const g = evaluateSwapPayAcquireGuidance(
      base({
        payRaw: 10_000_000n,
        payBalanceRaw: 10_000_000n,
        expectedSlippagePct: 8,
      })
    )
    expect(g.kind).toBe('high_impact')
    expect(g.message).toBe(SWAP_ACQUIRE_COPY.highImpact)
    expect(g.reduceToHuman).toBe('1')
  })

  it('does not treat ticker UST1 as the window token (T10)', () => {
    expect(isUst1PayAsset('UST1', UST1)).toBe(false)
    expect(isUst1PayAsset(UST1, UST1)).toBe(true)
    expect(isUst1PayAsset(UST1, '')).toBe(false)
  })
})

describe('acquire href + U1 copy (#678)', () => {
  it('allowlists only /ust1 and /wrap', () => {
    expect(isAllowedAcquireHref('/ust1?direction=deposit&amount=10')).toBe(true)
    expect(isAllowedAcquireHref('/wrap')).toBe(true)
    expect(isAllowedAcquireHref('https://evil.example/ust1')).toBe(false)
    expect(isAllowedAcquireHref('javascript:alert(1)')).toBe(false)
    expect(isAllowedAcquireHref('/ust1?next=https://evil')).toBe(false)
    expect(buildUst1DepositHref('10')).toBe('/ust1?direction=deposit&amount=10')
  })

  it('assertSecondaryMarketCopy on every retail string', () => {
    const strings = [
      SWAP_ACQUIRE_COPY.disconnectedQuoteOnly,
      SWAP_ACQUIRE_COPY.insufficientGeneric('UST1'),
      SWAP_ACQUIRE_COPY.insufficientUst1Window('50,000', '1,000'),
      SWAP_ACQUIRE_COPY.insufficientUst1OverWindow,
      SWAP_ACQUIRE_COPY.insufficientWrap('cLUNC'),
      SWAP_ACQUIRE_COPY.highImpact,
      SWAP_ACQUIRE_COPY.guideUst1,
      SWAP_ACQUIRE_COPY.guideWrap,
    ]
    for (const s of strings) assertSecondaryMarketCopy(s)
  })
})
