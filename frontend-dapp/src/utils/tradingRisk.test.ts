import { describe, expect, it, vi } from 'vitest'
import { tradingRiskFixture } from '@/test/tradingRiskFixtures'
import {
  assertTradingRiskForSubmit,
  evaluateTradingRisk,
  exactTradingRiskPredicates,
  tradingRiskBlocksSubmit,
} from './tradingRisk'

const B = 10n ** 36n
function predicates(movement: bigint, difference: bigint, favorable = false) {
  return exactTradingRiskPredicates({
    movement: String(movement),
    baseline: String(B),
    net: String(favorable ? B : B - difference),
    fair: String(favorable ? B - difference : B),
  })!
}
function sim() {
  const q = tradingRiskFixture()
  return { return_amount: q.estimated_amount_out!, executionMetadata: q.execution_metadata!, riskEvidence: q }
}

describe('shared exact trading risk #1560', () => {
  for (const orderOffset of [-1n, 0n, 1n])
    for (const totalOffset of [-1n, 0n, 1n]) {
      it(`compares one raw unit at order5 ${orderOffset} / total10 ${totalOffset}`, () => {
        const p = predicates(B / 20n + orderOffset, B / 10n + totalOffset)
        expect(p.orderWarning).toBe(orderOffset > 0n)
        expect(p.totalWarning).toBe(totalOffset > 0n)
        expect(p.expertRequired).toBe(false)
        expect(p.extreme).toBe(false)
      })
    }
  it('does not add fees to the 3.34 / 1.71 / 6.91 control', () => {
    const p = predicates((B * 334n) / 10000n, (B * 691n) / 10000n)
    expect(p.orderWarning || p.totalWarning).toBe(false)
  })
  it.each([false, true])('preserves symmetric >30 / >=99 and signed improvement (favorable=%s)', (favorable) => {
    expect(predicates(-B / 10n, 0n, favorable).orderWarning).toBe(false)
    for (const offset of [-1n, 0n, 1n]) {
      expect(predicates(-B / 10n, (B * 3n) / 10n + offset, favorable).expertRequired).toBe(offset > 0n)
      expect(predicates(-B / 10n, (B * 99n) / 100n + offset, favorable).extreme).toBe(offset >= 0n)
    }
  })
  it.each(['NaN', 'Infinity', '-1', '01', '0', '1e20', '340282366920938463463374607431768211456'])(
    'rejects invalid original denominators %s',
    (baseline) => {
      expect(exactTradingRiskPredicates({ movement: '1', baseline, fair: '100', net: '99' })).toBeNull()
    }
  )
  it('validates original selected economics before accepting display or labels', () => {
    const s = sim(),
      risk = evaluateTradingRisk(s)
    expect(risk.availability).toBe('available')
    expect(risk.reason).toBe('total')
    expect(risk.consentRequired).toBe(true)
    expect(tradingRiskBlocksSubmit(risk, false)).toBe(false)
    ;(s.riskEvidence.order_economics as Record<string, unknown>).order_movement = '0'
    expect(evaluateTradingRisk(s).availability).toBe('unavailable')
    expect(tradingRiskBlocksSubmit(evaluateTradingRisk(s), true)).toBe(true)
  })
  it('never supplies safe evidence from missing, mismatched, expired or forged observations', () => {
    const s = sim()
    expect(evaluateTradingRisk({ ...s, riskEvidence: undefined }).availability).toBe('unavailable')
    expect(evaluateTradingRisk({ ...s, return_amount: '99' }).availability).toBe('unavailable')
    const clock = vi.spyOn(Date, 'now').mockReturnValue(s.executionMetadata.expires_at_ms)
    expect(evaluateTradingRisk(s).availability).toBe('unavailable')
    clock.mockRestore()
  })
  it('binds exact risk evidence for consent and preserves it on an unchanged refresh', () => {
    const s = sim()
    expect(evaluateTradingRisk(structuredClone(s)).evidenceKey).toBe(evaluateTradingRisk(s).evidenceKey)
    const q = s.riskEvidence,
      a = q.economic_attribution as Record<string, unknown>
    q.spot_amount_out = a.fair_amount_out = '121'
    ;(a.components as Record<string, unknown>).reference_divergence = '21'
    ;(q.order_economics as Record<string, unknown>).reference_divergence = '21'
    expect(evaluateTradingRisk(s).evidenceKey).not.toBe(evaluateTradingRisk(sim()).evidenceKey)
  })
  it('rechecks fresh consent and refuses mutated risk after rendering even with an old acceptance', () => {
    const s = sim(),
      original = evaluateTradingRisk(s).evidenceKey
    expect(() => assertTradingRiskForSubmit(s, false, original, false)).toThrow('Confirm the price difference first.')
    expect(() => assertTradingRiskForSubmit(s, false, original, true)).not.toThrow()
    const q = s.riskEvidence,
      a = q.economic_attribution as Record<string, unknown>
    q.spot_amount_out = a.fair_amount_out = '121'
    ;(a.components as Record<string, unknown>).reference_divergence = '21'
    ;(q.order_economics as Record<string, unknown>).reference_divergence = '21'
    expect(() => assertTradingRiskForSubmit(s, false, original, true)).toThrow('Quote risk is unavailable')
  })
  it('binds original account and execution metadata against after-render mutation', () => {
    const s = sim(),
      original = evaluateTradingRisk(s).evidenceKey
    s.executionMetadata.trader = 'terra1different'
    expect(() => assertTradingRiskForSubmit(s, false, original, true)).toThrow('Quote risk is unavailable')
  })
})
