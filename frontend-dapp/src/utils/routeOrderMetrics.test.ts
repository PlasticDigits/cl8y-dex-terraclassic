import { describe, expect, it } from 'vitest'
import { localQuote } from '@/test/localRouteFixtures'
import { attachReferenceAttribution } from '@/test/routeEconomicFixtures'
import { formatOrderMetric, routeOrderMetrics } from './routeOrderMetrics'

function fixture() {
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
describe('selected-route order metrics #1500', () => {
  it('binds unused original maker terms when removing sell tax expands the fee-free book budget', () => {
    // Sell funding: gross200 = economic100 + tax100. Actual book budget100
    // consumes maker one at 1x only; fee-free gross200 also reaches maker two.
    const q = localQuote(undefined, undefined, '200', undefined, '99')
    q.spot_amount_out = '200'
    q.estimated_amount_out_net = '99'
    q.tax_kind = 'route_local_v1'
    q.tax_funding_version = 1
    q.router_hops_tax = true
    q.sell_tax_bps = 10000
    attachReferenceAttribution(q)
    const m = q.execution_metadata!,
      h = m.hops[0],
      a = q.economic_attribution as Record<string, unknown>
    m.policy = 'declared_hybrid_hop0'
    m.tax_funding_version = 1
    Object.assign(h, {
      economic_offer_amount: '100',
      sell_tax_amount: '100',
      sell_residue: '0',
      sell_tax_bps: 10000,
      buy_tax_bps: 0,
      buy_tax_amount: '0',
      post_hook_return_amount: '99',
      pool_return_amount: '0',
      book_return_net: '99',
      pool_input_actual: '0',
      book_offer_consumed: '100',
      commission_amount: '0',
      book_commission_amount: '1',
      reserve_in: '10000',
      reserve_out: '10000',
      book_side: 'bid',
      declared_hybrid: { pool_input: '0', book_input: '100', max_maker_fills: 2, book_start_hint: null },
    })
    h.book_terms = [
      { remaining: '100', price_atomics: '1000000000000000000' },
      { remaining: '100', price_atomics: '1000000000000000000' },
    ]
    ;(q.router_operations[0] as { terra_swap: { hybrid?: unknown } }).terra_swap.hybrid = h.declared_hybrid
    a.components = { wrapper_loss: '0', trading_cost: '101', market_movement: '0', reference_divergence: '0' }
    a.dominant_cause = 'trading_cost'
    const d: Record<string, unknown> = {
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
      baseline_amount_out: '200',
      fee_free_amount_out: '200',
      order_movement: '0',
      trading_fee: '1',
      tax_hook_cost: '100',
      wrapper_cost: '0',
      reference_divergence: '0',
      stages: [
        {
          reserve_in: '10000',
          reserve_out: '10000',
          input_amount: '200',
          output_amount: '200',
          book_input: '200',
          max_maker_fills: 2,
          book_side: 'bid',
          book: structuredClone(h.book_terms),
        },
      ],
    }
    q.order_economics = d
    expect(routeOrderMetrics(q).orderSlippagePercent).toBe(0)
    const stage = (d.stages as Record<string, unknown>[])[0]
    // Actual original consumed/gross output and all guards remain unchanged.
    stage.book = [
      { remaining: '100', price_atomics: '1000000000000000000' },
      { remaining: '200', price_atomics: '2000000000000000000' },
    ]
    stage.output_amount = d.fee_free_amount_out = '300'
    d.order_movement = '-100'
    expect(routeOrderMetrics(q).orderSlippagePercent).toBeNull()
    expect(h.book_terms[1]).toEqual({ remaining: '100', price_atomics: '1000000000000000000' })
  })
  it.each([1n, 2n])('independently replays original book pricing at %s× spot and keeps fees separate', (multiplier) => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>,
      m = q.execution_metadata!,
      h = m.hops[0]
    const grossOutput = 100n * multiplier,
      fee = multiplier,
      net = grossOutput - fee
    q.estimated_amount_out = q.estimated_amount_out_net = String(net)
    h.return_amount = h.book_return_net = String(net)
    h.pool_return_amount = '0'
    h.commission_amount = '0'
    h.book_commission_amount = String(fee)
    h.pool_input_actual = '0'
    h.book_offer_consumed = '100'
    h.spread_amount = '0'
    h.declared_hybrid = { pool_input: '0', book_input: '100', max_maker_fills: 1, book_start_hint: null }
    ;(q.router_operations![0] as { terra_swap: { hybrid?: unknown } }).terra_swap.hybrid = h.declared_hybrid
    m.policy = 'declared_hybrid_hop0'
    d.policy = m.policy
    const a = q.economic_attribution as Record<string, unknown>
    a.net_amount_out = String(net)
    a.components = {
      wrapper_loss: '0',
      trading_cost: String(fee),
      market_movement: String(100n - grossOutput),
      reference_divergence: '20',
    }
    a.dominant_cause = multiplier === 1n ? 'reference_divergence' : 'market_movement'
    d.router_operations = q.router_operations
    d.fee_free_amount_out = String(grossOutput)
    d.order_movement = String(100n - grossOutput)
    d.trading_fee = String(fee)
    d.stages = [
      {
        reserve_in: '10000',
        reserve_out: '10000',
        input_amount: '100',
        output_amount: String(grossOutput),
        book_input: '100',
        max_maker_fills: 1,
        book_side: 'bid',
        book: [{ remaining: String(grossOutput), price_atomics: String(10n ** 18n * multiplier) }],
      },
    ]
    h.book_terms = structuredClone((d.stages as { book: { remaining: string; price_atomics: string }[] }[])[0].book)
    expect(routeOrderMetrics(q).orderSlippagePercent).toBe(multiplier === 1n ? 0 : -100)
    expect(routeOrderMetrics(q).tradingFeePercent).toBe(Number(multiplier))
    ;(d.stages as Record<string, unknown>[])[0].output_amount = String(grossOutput + 1n)
    d.fee_free_amount_out = String(grossOutput + 1n)
    d.order_movement = String(99n - grossOutput)
    expect(routeOrderMetrics(q).orderSlippagePercent).toBeNull()
  })
  it('independently checks the selected reserve denominator and CP output, separately from fair reference and fees', () => {
    expect(routeOrderMetrics(fixture())).toEqual({
      orderSlippagePercent: 1,
      tradingFeePercent: 1,
      costs: { tradingFeeRaw: '1', taxHookRaw: '0', wrapperRaw: '0', referenceDivergenceRaw: '20' },
    })
  })
  it.each([
    'method',
    'version',
    'generation',
    'observed_at_ms',
    'expires_at_ms',
    'input_amount',
    'policy',
    'max_spread',
    'router_operations',
    'baseline_amount_out',
    'fee_free_amount_out',
    'order_movement',
    'trading_fee',
    'tax_hook_cost',
    'wrapper_cost',
    'reference_divergence',
  ])('rejects malformed/mismatched %s rather than showing zero', (key) => {
    const q = fixture()
    ;(q.order_economics as Record<string, unknown>)[key] = 'tampered'
    expect(routeOrderMetrics(q).orderSlippagePercent).toBeNull()
    expect(routeOrderMetrics(q).tradingFeePercent).toBeNull()
  })
  it.each(['-0', '01', '1e3', '1.2', '340282366920938463463374607431768211456'])(
    'rejects noncanonical raw %s',
    (raw) => {
      const q = fixture()
      ;(q.order_economics as Record<string, unknown>).trading_fee = raw
      expect(routeOrderMetrics(q).costs).toBeNull()
    }
  )
  it('rejects a forged baseline even when the attacker reconciles its numerator', () => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>
    d.baseline_amount_out = '99'
    d.order_movement = '0'
    expect(routeOrderMetrics(q).orderSlippagePercent).toBeNull()
  })
  it('binds internally consistent forged reserve denominators to the original attribution bridge', () => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>,
      s = (d.stages as Record<string, unknown>[])[0]
    s.reserve_in = '100'
    s.reserve_out = '200'
    s.output_amount = '100'
    d.baseline_amount_out = '200'
    d.fee_free_amount_out = '100'
    d.order_movement = '100'
    expect(routeOrderMetrics(q).costs).toBeNull()
  })
  it('binds original one-hop precommission output even if forged depth preserves the spot denominator', () => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>,
      s = (d.stages as Record<string, unknown>[])[0]
    s.reserve_in = s.reserve_out = '100'
    s.output_amount = '50'
    d.fee_free_amount_out = '50'
    d.order_movement = '50'
    expect(routeOrderMetrics(q).costs).toBeNull()
  })
  it('rejects a forged zero movement counterfactual even with matching raw outputs', () => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>
    d.fee_free_amount_out = '100'
    d.order_movement = '0'
    ;(d.stages as Record<string, unknown>[])[0].output_amount = '100'
    expect(routeOrderMetrics(q).orderSlippagePercent).toBeNull()
  })
  it('displays truly unavailable metrics and preserves tiny favorable signs', () => {
    expect(routeOrderMetrics(localQuote()).orderSlippagePercent).toBeNull()
    expect(formatOrderMetric(null)).toBe('Unavailable')
    expect(formatOrderMetric(-0.00001)).toBe('>-0.01%')
    expect(formatOrderMetric(-1.5)).toBe('-1.50%')
  })
  it('binds original expiry and refuses unsupported extra fields', () => {
    const q = fixture(),
      d = q.order_economics as Record<string, unknown>
    q.execution_metadata!.expires_at_ms = Date.now() - 1
    d.expires_at_ms = q.execution_metadata!.expires_at_ms
    expect(routeOrderMetrics(q).costs).toBeNull()
    const extra = fixture()
    ;(extra.order_economics as Record<string, unknown>).actor = 'private'
    expect(routeOrderMetrics(extra).costs).toBeNull()
  })
})
