import { describe, expect, it } from 'vitest'
import { localQuote } from '@/test/localRouteFixtures'
import { attachReferenceAttribution } from '@/test/routeEconomicFixtures'
import { routeEconomicExplanation, UNAVAILABLE_ECONOMIC_EXPLANATION } from './routeEconomicExplanation'

function fixture() {
  const quote = localQuote()
  quote.spot_amount_out = '1200000'
  return attachReferenceAttribution(quote)
}
describe('bounded advisory economic explanation #1483', () => {
  it('explains independently reconciled reference divergence without an extra percent', () => {
    expect(routeEconomicExplanation(fixture())).toBe('Different pricing')
  })
  it.each([
    ['wrapper_loss', ['150000', '50000', '0', '0'], 'Wrapping costs'],
    ['trading_cost', ['0', '160000', '40000', '0'], 'Trading costs'],
    ['market_movement', ['0', '50000', '250000', '-100000'], 'Selected pool price movement'],
    ['mixed', ['100000', '100000', '0', '0'], 'Combined costs and pricing'],
  ])('derives %s from signed same-denomination terms', (cause, values, copy) => {
    const q = fixture(),
      a = q.economic_attribution as Record<string, unknown>
    a.dominant_cause = cause
    a.components = Object.fromEntries(
      ['wrapper_loss', 'trading_cost', 'market_movement', 'reference_divergence'].map((key, i) => [key, values[i]])
    )
    expect(routeEconomicExplanation(q)).toBe(copy)
  })
  it.each([
    'generation',
    'observed_at_ms',
    'expires_at_ms',
    'fair_amount_out',
    'net_amount_out',
    'dominant_cause',
    'method',
    'version',
  ])('rejects tampered %s', (key) => {
    const q = fixture()
    ;(q.economic_attribution as Record<string, unknown>)[key] = '<script>private provider</script>'
    expect(routeEconomicExplanation(q)).toBe(UNAVAILABLE_ECONOMIC_EXPLANATION)
  })
  it.each(['-0', '1.1', '1e6', '9'.repeat(100), '340282366920938463463374607431768211456'])(
    'rejects noncanonical/oversized component %s',
    (raw) => {
      const q = fixture()
      ;(
        (q.economic_attribution as Record<string, unknown>).components as Record<string, unknown>
      ).reference_divergence = raw
      expect(routeEconomicExplanation(q)).toBe(UNAVAILABLE_ECONOMIC_EXPLANATION)
    }
  )
  it('does not invent costs for an exact zero-component quote', () => {
    expect(routeEconomicExplanation(attachReferenceAttribution(localQuote()))).toBe('Matches reference prices')
  })
  it.each(['expired', 'future', 'extended', 'unsafe', 'net_mismatch'])('rejects %s diagnostic binding', (kind) => {
    const q = fixture()
    const a = q.economic_attribution as Record<string, unknown>,
      metadata = q.execution_metadata!
    if (kind === 'net_mismatch') q.estimated_amount_out_net = '1'
    else if (kind === 'expired') a.expires_at_ms = metadata.expires_at_ms = Date.now() - 1
    else if (kind === 'future') a.observed_at_ms = metadata.observed_at_ms = Date.now() + 1000
    else if (kind === 'extended') a.expires_at_ms = metadata.expires_at_ms = Date.now() + 20001
    else a.observed_at_ms = metadata.observed_at_ms = Number.MAX_SAFE_INTEGER + 1
    expect(routeEconomicExplanation(q)).toBe(UNAVAILABLE_ECONOMIC_EXPLANATION)
  })
  it('rejects nonreconciliation, extra private keys, forged dominant cause and unavailable evidence', () => {
    for (const mutation of [
      (a: Record<string, unknown>) => {
        a.components = { wrapper_loss: '0', trading_cost: '0', market_movement: '0', reference_divergence: '1' }
      },
      (a: Record<string, unknown>) => {
        a.actor = 'private'
      },
      (a: Record<string, unknown>) => {
        a.dominant_cause = 'market_movement'
      },
      (a: Record<string, unknown>) => {
        a.status = 'unavailable'
      },
    ]) {
      const q = fixture()
      mutation(q.economic_attribution as Record<string, unknown>)
      expect(routeEconomicExplanation(q)).toBe(UNAVAILABLE_ECONOMIC_EXPLANATION)
    }
    expect(routeEconomicExplanation(localQuote())).toBe(UNAVAILABLE_ECONOMIC_EXPLANATION)
  })
})
