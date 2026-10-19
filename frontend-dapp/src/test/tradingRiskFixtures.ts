import { localQuote } from './localRouteFixtures'
import { attachReferenceAttribution } from './routeEconomicFixtures'

export function tradingRiskFixture() {
  // Independently hand-computed CP: floor(100*10000/(10000+100))=99.
  // Commission 1 yields 98; the fee-free spot denominator is 100, not fair=120.
  const q = localQuote(undefined, undefined, '100', undefined, '98')
  q.spot_amount_out = '120'
  q.estimated_amount_out_net = '98'
  attachReferenceAttribution(q)
  const a = q.economic_attribution as Record<string, unknown>
  a.components = { wrapper_loss: '0', trading_cost: '1', market_movement: '1', reference_divergence: '20' }
  const m = q.execution_metadata!
  m.hops[0].pool_return_amount = '98'
  m.hops[0].commission_amount = '1'
  m.hops[0].book_commission_amount = '0'
  m.hops[0].spread_amount = '1'
  m.hops[0].reserve_in = '10000'
  m.hops[0].reserve_out = '10000'
  m.hops[0].book_side = 'bid'
  m.hops[0].book_terms = []
  q.order_economics = {
    version: 1,
    status: 'available',
    method: 'selected_route_fee_free_v1',
    generation: m.generation,
    observed_at_ms: m.observed_at_ms,
    expires_at_ms: m.expires_at_ms,
    input_amount: m.input_amount,
    policy: m.policy,
    max_spread: m.max_spread,
    router_operations: q.router_operations,
    stages: [
      {
        reserve_in: '10000',
        reserve_out: '10000',
        input_amount: '100',
        output_amount: '99',
        book_input: '0',
        max_maker_fills: 1,
        book_side: 'bid',
        book: [],
      },
    ],
    baseline_amount_out: '100',
    fee_free_amount_out: '99',
    order_movement: '1',
    trading_fee: '1',
    tax_hook_cost: '0',
    wrapper_cost: '0',
    reference_divergence: '20',
  }
  return q
}
