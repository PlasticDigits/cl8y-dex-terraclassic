import type { IndexerRouteSolveResponse } from '@/types'

/** Owned public synthetic diagnostics; no wallet/actor fields. */
export function attachReferenceAttribution(quote: IndexerRouteSolveResponse): IndexerRouteSolveResponse {
  const metadata = quote.execution_metadata!
  quote.economic_attribution = {
    version: 1,
    status: 'available',
    method: 'spot_bridge_v1',
    generation: metadata.generation,
    observed_at_ms: metadata.observed_at_ms,
    expires_at_ms: metadata.expires_at_ms,
    fair_amount_out: quote.spot_amount_out,
    net_amount_out: quote.estimated_amount_out,
    components: {
      wrapper_loss: '0',
      trading_cost: '0',
      market_movement: '0',
      reference_divergence: (BigInt(quote.spot_amount_out!) - BigInt(quote.estimated_amount_out!)).toString(),
    },
    dominant_cause: quote.spot_amount_out === quote.estimated_amount_out ? 'mixed' : 'reference_divergence',
  }
  return quote
}

/** Exact synthetic zero-movement CP evidence for plain one-hop zero-fee quotes.
 * Reserve choice independently makes floor spot == floor CP == actual net.
 * Use only owned fixtures without taxes/hooks/wrappers/manual book execution.
 */
export function attachZeroMovementRiskFixture(quote: IndexerRouteSolveResponse): IndexerRouteSolveResponse {
  const m = quote.execution_metadata!
  const scale = 10n ** 18n,
    max = (1n << 128n) - 1n
  let previous = BigInt(m.input_amount),
    spot = previous
  const stages = []
  for (const h of m.hops) {
    if (h.commission_amount !== '0' || (h.book_commission_amount != null && h.book_commission_amount !== '0'))
      return quote
    const input = BigInt(h.economic_offer_amount ?? h.offer_amount),
      net = BigInt(h.return_amount)
    const rin = input * scale,
      rout = (net + 1n) * scale - 1n
    const book = BigInt(h.book_offer_consumed ?? '0'),
      pool = BigInt(h.pool_input_actual ?? h.offer_amount)
    if (
      previous !== input ||
      net >= scale ||
      input <= 0n ||
      rin > max ||
      rout > max ||
      (input * rout) / rin !== net ||
      (pool * rout) / (rin + pool) !== BigInt(h.pool_return_amount ?? '0') ||
      BigInt(h.book_return_net ?? '0') !== book ||
      pool + book !== input
    )
      throw new Error('Fixture CP arithmetic/scope')
    const rows = book > 0n ? [{ remaining: String(book), price_atomics: String(scale) }] : []
    Object.assign(h, {
      reserve_in: String(rin),
      reserve_out: String(rout),
      book_side: 'bid',
      book_terms: rows,
      book_commission_amount: '0',
    })
    stages.push({
      reserve_in: String(rin),
      reserve_out: String(rout),
      input_amount: String(input),
      output_amount: String(net),
      book_input: String(book),
      max_maker_fills: h.declared_hybrid?.max_maker_fills ?? 1,
      book_side: 'bid',
      book: rows,
    })
    previous = net
    spot = (spot * rout) / rin
  }
  if (previous !== BigInt(quote.estimated_amount_out!) || spot !== previous) throw new Error('Fixture zero-cost scope')
  const ref = String(BigInt(quote.spot_amount_out!) - previous)
  quote.order_economics = {
    version: 1,
    status: 'available',
    method: 'selected_route_fee_free_v1',
    generation: m.generation,
    observed_at_ms: m.observed_at_ms,
    expires_at_ms: m.expires_at_ms,
    input_amount: m.input_amount,
    policy: m.policy,
    max_spread: m.max_spread,
    router_operations: quote.router_operations,
    baseline_amount_out: String(spot),
    fee_free_amount_out: String(previous),
    order_movement: '0',
    trading_fee: '0',
    tax_hook_cost: '0',
    wrapper_cost: '0',
    reference_divergence: ref,
    stages,
  }
  return attachReferenceAttribution(quote)
}
