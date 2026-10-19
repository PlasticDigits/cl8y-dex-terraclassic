import type { IndexerRouteExecutionMetadata, IndexerRouteSolveResponse } from '@/types'
import { validateWrapperOnlyQuote } from './localWrapperOnlyQuote'
import { routeOrderMetrics } from './routeOrderMetrics'
import { preciseFairDeviationPercent, formatTradingPriceDifference } from './swapRouteSlippage'

export type TradingRiskQuote = {
  return_amount: string
  executionMetadata?: IndexerRouteExecutionMetadata
  /** Original response, retained only after the ordinary execution validator succeeds. */
  riskEvidence?: IndexerRouteSolveResponse
}
export type TradingRisk = {
  availability: 'available' | 'unavailable'
  reason: 'none' | 'order' | 'total' | 'both' | 'unavailable'
  orderWarning: boolean
  totalWarning: boolean
  consentRequired: boolean
  expertRequired: boolean
  extreme: boolean
  orderPercent: number | null
  totalPercent: number | null
  evidenceKey: string | null
}
const unavailable = (): TradingRisk => ({
  availability: 'unavailable',
  reason: 'unavailable',
  orderWarning: false,
  totalWarning: false,
  consentRequired: false,
  expertRequired: false,
  extreme: false,
  orderPercent: null,
  totalPercent: null,
  evidenceKey: null,
})

/** #1560: checked original integer economics determine every policy transition.
 * Display percentages and legacy worst-hop/aggregate labels never grant permission.
 * Re-evaluate at submission so original expiry cannot be renewed by review.
 */
export function evaluateTradingRisk(sim: TradingRiskQuote | null | undefined): TradingRisk {
  const q = sim?.riskEvidence
  if (
    !sim ||
    !q ||
    !q.execution_metadata ||
    !sim.executionMetadata ||
    q.estimated_amount_out !== sim.return_amount ||
    JSON.stringify(q.execution_metadata) !== JSON.stringify(sim.executionMetadata)
  )
    return unavailable()
  const metrics = routeOrderMetrics(q)
  let movement: unknown, baseline: unknown
  if (q.execution_metadata.policy === 'wrapper_only' && q.order_economics == null) {
    // An admitted equal-principal mapper has no selected pool/book market movement.
    // Revalidate its exact mapping, original clocks, identities, positive floors
    // and protocol capability. Fees remain exclusively in the total comparison.
    const m = q.execution_metadata
    try {
      validateWrapperOnlyQuote(q, {
        simRaw: m.input_amount,
        maxSpreadStr: m.max_spread,
        slippageTolerancePercent: Number(m.max_spread) * 100,
        trader: m.trader ?? undefined,
        sender: m.sender ?? undefined,
      })
      if (m.execution_supported !== true) return unavailable()
    } catch {
      return unavailable()
    }
    movement = '0'
    baseline = m.input_amount
  } else {
    if (metrics.orderSlippagePercent == null || !q.order_economics) return unavailable()
    // routeOrderMetrics independently replayed/bound original integer economics,
    // fair/net reconciliation, generation, observation and expiry.
    const d = q.order_economics as Record<string, unknown>
    movement = d.order_movement
    baseline = d.baseline_amount_out
  }
  const predicates = exactTradingRiskPredicates({
    movement,
    baseline,
    net: q.estimated_amount_out,
    fair: q.spot_amount_out,
  })
  if (!predicates) return unavailable()
  const { orderWarning, totalWarning, expertRequired, extreme } = predicates
  return {
    availability: 'available',
    reason: orderWarning ? (totalWarning ? 'both' : 'order') : totalWarning ? 'total' : 'none',
    orderWarning,
    totalWarning,
    consentRequired: orderWarning || totalWarning,
    expertRequired,
    extreme,
    orderPercent: metrics.orderSlippagePercent ?? 0,
    totalPercent: Number(preciseFairDeviationPercent(q.estimated_amount_out!, q.spot_amount_out!)),
    evidenceKey: JSON.stringify([
      q.order_economics,
      q.economic_attribution,
      q.token_in,
      q.token_out,
      q.execution_metadata,
      q.router_operations,
      q.estimated_amount_out,
      q.spot_amount_out,
    ]),
  }
}

/** Arithmetic only. Callers must independently validate/bind the economics first. */
export function exactTradingRiskPredicates(input: {
  movement: unknown
  baseline: unknown
  net: unknown
  fair: unknown
}) {
  const parse = (value: unknown, signed = false): bigint | null => {
    if (typeof value !== 'string' || !(signed ? /^(?:0|-?[1-9]\d{0,38})$/ : /^[1-9]\d{0,38}$/).test(value)) return null
    const n = BigInt(value),
      max = (1n << 128n) - 1n
    return n >= -max && n <= max ? n : null
  }
  const movement = parse(input.movement, true),
    baseline = parse(input.baseline),
    net = parse(input.net),
    fair = parse(input.fair)
  if (movement == null || baseline == null || net == null || fair == null || movement >= baseline) return null
  const denominator = net > fair ? net : fair
  const difference = net > fair ? net - fair : fair - net
  return {
    orderWarning: movement * 100n > baseline * 5n,
    totalWarning: difference * 100n > denominator * 10n,
    expertRequired: difference * 100n > denominator * 30n,
    extreme: difference * 100n >= denominator * 99n,
  }
}

export function tradingRiskBlocksSubmit(risk: TradingRisk, expertMode: boolean): boolean {
  return risk.availability !== 'available' || risk.extreme || (risk.expertRequired && !expertMode)
}

export function tradingRiskNotice(risk: TradingRisk): string {
  if (risk.availability !== 'available') return 'Quote risk is unavailable. Refresh the quote.'
  const order = `Order slippage ${formatTradingPriceDifference(risk.orderPercent!)}%`
  const total = `Fair-reference deviation ${formatTradingPriceDifference(risk.totalPercent!)}%`
  return risk.reason === 'both' ? `${order}; ${total.toLowerCase()}` : risk.reason === 'order' ? order : total
}

/** Final offline revalidation also rejects evidence changed after the rendered review. */
export function assertTradingRiskForSubmit(
  sim: TradingRiskQuote | null | undefined,
  expertMode: boolean,
  renderedEvidenceKey: string | null,
  consentAccepted: boolean
): void {
  const current = evaluateTradingRisk(sim)
  if (current.evidenceKey !== renderedEvidenceKey || tradingRiskBlocksSubmit(current, expertMode))
    throw new Error('Quote risk is unavailable or exceeds protection.')
  if (current.consentRequired && !consentAccepted) throw new Error('Confirm the price difference first.')
}
