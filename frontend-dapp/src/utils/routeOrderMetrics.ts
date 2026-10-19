import type { IndexerRouteSolveResponse } from '@/types'
import { routeEconomicExplanation, UNAVAILABLE_ECONOMIC_EXPLANATION } from './routeEconomicExplanation'

export type RouteOrderCosts = {
  tradingFeeRaw: string
  taxHookRaw: string
  wrapperRaw: string
  referenceDivergenceRaw: string
}
export type RouteOrderMetrics = {
  orderSlippagePercent: number | null
  tradingFeePercent: number | null
  costs: RouteOrderCosts | null
}
const MAX = (1n << 128n) - 1n
function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : null
}
function raw(value: unknown, signed = false): bigint | null {
  if (typeof value !== 'string' || !(signed ? /^(?:0|-?[1-9]\d{0,38})$/ : /^(?:0|[1-9]\d{0,38})$/).test(value))
    return null
  const n = BigInt(value)
  return n >= -MAX && n <= MAX ? n : null
}
function keys(value: Record<string, unknown>, expected: string[]) {
  return Object.keys(value).length === expected.length && Object.keys(value).every((key) => expected.includes(key))
}
function percent(n: bigint, d: bigint): number {
  // Checked integer ratio before conversion; preserve favorable signs.
  const result = (Number(n) / Number(d)) * 100
  // Keep a one-raw-unit original crossing visible even when IEEE conversion
  // collapses that ratio onto the boundary. Policy itself uses only integers.
  for (const threshold of [5, 10, 30, 99]) {
    const delta = n * 100n - BigInt(threshold) * d
    if (delta > 0n && result <= threshold) return threshold + Number.EPSILON * threshold
    if (delta < 0n && result >= threshold) return threshold - Number.EPSILON * threshold
  }
  return result
}
function replayBook(
  rows: unknown[],
  side: 'bid' | 'ask',
  budget: bigint,
  maxFills: number
): { consumed: bigint; output: bigint } | null {
  let left = budget,
    bookOutput = 0n,
    used = 0
  const scale = 10n ** 18n
  for (const value of rows) {
    const row = record(value)
    if (!row || !keys(row, ['remaining', 'price_atomics'])) return null
    const remaining = raw(row.remaining),
      price = raw(row.price_atomics)
    if (remaining == null || remaining <= 0n || price == null || price <= 0n) return null
    const inverse = (scale * scale) / price
    if (inverse <= 0n || inverse > MAX) return null
    const capacity = (remaining * inverse) / scale
    if (
      side === 'bid'
        ? capacity === 0n ||
          capacity > MAX ||
          (capacity * price) / scale === 0n ||
          (capacity * price) / scale > remaining
        : remaining * price < scale
    )
      return null
    if (used >= maxFills || left === 0n) continue
    const budgetFill = side === 'bid' ? capacity : (left * inverse) / scale
    if (budgetFill > MAX) return null
    const fill =
      side === 'bid' ? (left < budgetFill ? left : budgetFill) : remaining < budgetFill ? remaining : budgetFill
    const cost = (fill * price) / scale
    if (fill === 0n || cost === 0n) continue
    if (cost > MAX || (side === 'bid' ? cost > remaining : cost > left)) return null
    left -= side === 'bid' ? fill : cost
    bookOutput += side === 'bid' ? cost : fill
    if (bookOutput > MAX) return null
    used++
  }
  return { consumed: budget - left, output: bookOutput }
}

const unavailable = (): RouteOrderMetrics => ({ orderSlippagePercent: null, tradingFeePercent: null, costs: null })

/** Independently checked selected-route original fee-free economics. Display
 * numbers alone grant no signing permission; TR1560 requires the original checked
 * evidence for risk classification and blocks unavailable evidence.
 */
export function routeOrderMetrics(quote: IndexerRouteSolveResponse): RouteOrderMetrics {
  const d = record(quote.order_economics),
    m = quote.execution_metadata
  if (
    !d ||
    !m ||
    !Array.isArray(m.hops) ||
    m.hops.length > 4 ||
    !Array.isArray(quote.router_operations) ||
    quote.router_operations.length !== m.hops.length ||
    routeEconomicExplanation(quote) === UNAVAILABLE_ECONOMIC_EXPLANATION ||
    !keys(d, [
      'version',
      'status',
      'method',
      'generation',
      'observed_at_ms',
      'expires_at_ms',
      'input_amount',
      'policy',
      'max_spread',
      'router_operations',
      'stages',
      'baseline_amount_out',
      'fee_free_amount_out',
      'order_movement',
      'trading_fee',
      'tax_hook_cost',
      'wrapper_cost',
      'reference_divergence',
    ]) ||
    d.version !== 1 ||
    d.status !== 'available' ||
    d.method !== 'selected_route_fee_free_v1' ||
    ['generation', 'observed_at_ms', 'expires_at_ms', 'input_amount', 'policy', 'max_spread'].some(
      (key) => d[key] !== (m as unknown as Record<string, unknown>)[key]
    ) ||
    JSON.stringify(d.router_operations) !== JSON.stringify(quote.router_operations) ||
    !Array.isArray(d.stages) ||
    d.stages.length !== m.hops.length ||
    d.stages.length > 4
  )
    return unavailable()
  const baseline = raw(d.baseline_amount_out),
    output = raw(d.fee_free_amount_out),
    movement = raw(d.order_movement, true),
    fees = raw(d.trading_fee),
    taxes = raw(d.tax_hook_cost),
    wrapper = raw(d.wrapper_cost),
    reference = raw(d.reference_divergence, true)
  if (
    baseline == null ||
    baseline <= 0n ||
    output == null ||
    output <= 0n ||
    movement == null ||
    fees == null ||
    taxes == null ||
    wrapper == null ||
    reference == null ||
    baseline - output !== movement
  )
    return unavailable()
  const stages = d.stages.map(record)
  const input = raw(m.input_amount)
  if (input == null || input <= 0n) return unavailable()
  let spot = input,
    previous = input
  for (let i = 0; i < stages.length; i++) {
    const s = stages[i],
      hop = m.hops[i]
    if (!hop || typeof hop !== 'object') return unavailable()
    if (
      !s ||
      !keys(s, [
        'reserve_in',
        'reserve_out',
        'input_amount',
        'output_amount',
        'book_input',
        'max_maker_fills',
        'book_side',
        'book',
      ])
    )
      return unavailable()
    const rin = raw(s.reserve_in),
      rout = raw(s.reserve_out),
      amount = raw(s.input_amount),
      net = raw(s.output_amount),
      book = raw(s.book_input)
    if (
      rin == null ||
      rin <= 0n ||
      rout == null ||
      rout <= 0n ||
      amount !== previous ||
      net == null ||
      net <= 0n ||
      book == null ||
      book > previous ||
      !Number.isInteger(s.max_maker_fills) ||
      Number(s.max_maker_fills) < 1 ||
      Number(s.max_maker_fills) > 100
    )
      return unavailable()
    const budget = hop.declared_hybrid ? raw(hop.declared_hybrid.book_input) : 0n
    const economic = raw(hop.economic_offer_amount ?? hop.offer_amount)
    if (
      budget == null ||
      economic == null ||
      economic <= 0n ||
      book !== (previous * budget) / economic ||
      s.max_maker_fills !== (hop.declared_hybrid?.max_maker_fills ?? 1)
    )
      return unavailable()
    if (
      !Array.isArray(s.book) ||
      s.book.length > 500 ||
      !Array.isArray(hop.book_terms) ||
      hop.book_terms.length > 500 ||
      JSON.stringify(hop.book_terms) !== JSON.stringify(s.book) ||
      (s.book_side !== 'bid' && s.book_side !== 'ask') ||
      (book === 0n && s.book.length !== 0)
    )
      return unavailable()
    if (raw(hop.reserve_in) !== rin || raw(hop.reserve_out) !== rout || hop.book_side !== s.book_side)
      return unavailable()
    // Independently replay sanitized original maker escrow/price terms. No
    // actor, order id or remote lookup is needed. Unspent book residue goes to CP.
    const freeBook = replayBook(s.book, s.book_side, book, Number(s.max_maker_fills))
    const actualBook = replayBook(s.book, s.book_side, budget, Number(s.max_maker_fills))
    const actualPoolInput = raw(hop.pool_input_actual),
      actualPoolNet = raw(hop.pool_return_amount),
      actualPoolFee = raw(hop.commission_amount),
      actualSpread = raw(hop.spread_amount),
      actualBookNet = raw(hop.book_return_net),
      actualBookFee = raw(hop.book_commission_amount),
      actualConsumed = raw(hop.book_offer_consumed)
    if (
      !freeBook ||
      !actualBook ||
      actualPoolInput == null ||
      actualPoolNet == null ||
      actualPoolFee == null ||
      actualSpread == null ||
      actualBookNet == null ||
      actualBookFee == null ||
      actualConsumed == null ||
      actualBook.consumed !== actualConsumed ||
      actualBook.output !== actualBookNet + actualBookFee ||
      actualPoolInput + actualConsumed !== economic ||
      actualPoolNet + actualPoolFee > MAX ||
      (actualPoolInput * rout) / (rin + actualPoolInput) !== actualPoolNet + actualPoolFee ||
      (actualPoolInput * rout) / rin - (actualPoolNet + actualPoolFee) !== actualSpread
    )
      return unavailable()
    const poolInput = previous - freeBook.consumed
    if (rin + poolInput > MAX || (poolInput * rout) / rin > MAX) return unavailable()
    if (net !== freeBook.output + (poolInput * rout) / (rin + poolInput)) return unavailable()
    spot = (spot * rout) / rin
    if (spot > MAX) return unavailable()
    previous = net
  }
  if (spot !== baseline || previous !== output || (stages.length === 0 && m.policy !== 'wrapper_only'))
    return unavailable()
  const a = record(quote.economic_attribution),
    c = record(a?.components)
  if (
    !c ||
    raw(c.trading_cost) !== fees + taxes ||
    raw(c.wrapper_loss) !== wrapper ||
    raw(c.reference_divergence, true) !== reference ||
    raw(quote.spot_amount_out) == null ||
    raw(quote.spot_amount_out)! - reference !== baseline
  )
    return unavailable()
  // Reconcile exact actual commissions in final-output units with suffix floors.
  const project = (value: bigint, start: number) => {
    for (let i = start; i < stages.length; i++) {
      value = (value * raw(stages[i]!.reserve_out)!) / raw(stages[i]!.reserve_in)!
      if (value > MAX) return null
    }
    return value
  }
  let checkedFees = 0n
  for (let i = 0; i < m.hops.length; i++) {
    const h = m.hops[i],
      poolFee = raw(h.commission_amount),
      bookFee = raw(h.book_commission_amount),
      post = raw(h.pool_return_amount),
      bookNet = raw(h.book_return_net)
    if (
      poolFee == null ||
      bookFee == null ||
      post == null ||
      bookNet == null ||
      post + bookNet + poolFee + bookFee > MAX
    )
      return unavailable()
    const before = project(post + bookNet + poolFee + bookFee, i + 1),
      after = project(post + bookNet, i + 1)
    if (before == null || after == null || before < after) return unavailable()
    // A one-hop counterfactual with unchanged gross/economic input must equal
    // the original output before commissions, independently of advisory depth
    // or maker prices. Later hook/buy/unwrap losses do not affect this boundary.
    if (
      m.hops.length === 1 &&
      raw(h.economic_offer_amount ?? h.offer_amount) === input &&
      output !== post + bookNet + poolFee + bookFee
    )
      return unavailable()
    checkedFees += before - after
  }
  if (fees !== checkedFees) return unavailable()
  return {
    orderSlippagePercent: percent(movement, baseline),
    tradingFeePercent: percent(fees, baseline),
    costs: {
      tradingFeeRaw: String(fees),
      taxHookRaw: String(taxes),
      wrapperRaw: String(wrapper),
      referenceDivergenceRaw: String(reference),
    },
  }
}

export function formatOrderMetric(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return 'Unavailable'
  if (value !== 0 && Math.abs(value) < 0.01) return value < 0 ? '>-0.01%' : '<0.01%'
  return `${value.toFixed(2)}%`
}
