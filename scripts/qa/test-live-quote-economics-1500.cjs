const { test } = require("node:test"),
  assert = require("node:assert/strict");
const { validate } = require("./live-quote-economics-1500.cjs");
function fixture() {
  const S = 10n ** 18n,
    u = S / 20n,
    N = 1n,
    D = 99n,
    num = (N * S + D * u) * S,
    den = D * (S - u),
    ceil = (num + den - 1n) / den;
  const h = {
    pair: "owned",
    offer_token: "in",
    ask_token: "out",
    offer_amount: "100",
    economic_offer_amount: "100",
    pool_input_actual: "100",
    book_offer_consumed: "0",
    pool_return_amount: "98",
    commission_amount: "1",
    book_return_net: "0",
    book_commission_amount: "0",
    spread_amount: "1",
    return_amount: "98",
    min_return: "94",
    declared_hybrid: null,
    reserve_in: "10000",
    reserve_out: "10000",
    book_side: "bid",
    book_terms: [],
  };
  const op = {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: "in" } },
      ask_asset_info: { token: { contract_addr: "out" } },
      min_return: "94",
      hybrid: null,
    },
  };
  const m = {
    version: 1,
    policy: "pool_only",
    protection_policy: "quote_relative_v1",
    generation: "owned-original",
    input_amount: "100",
    max_spread: "0.05",
    execution_market_spread: "0." + ceil.toString().padStart(18, "0"),
    minimum_receive: "94",
    observed_at_ms: 1000,
    expires_at_ms: 21000,
    hops: [h],
  };
  const a = {
    version: 1,
    status: "available",
    method: "spot_bridge_v1",
    generation: m.generation,
    observed_at_ms: m.observed_at_ms,
    expires_at_ms: m.expires_at_ms,
    fair_amount_out: "120",
    net_amount_out: "98",
    components: {
      wrapper_loss: "0",
      trading_cost: "1",
      market_movement: "1",
      reference_divergence: "20",
    },
  };
  const d = {
    version: 1,
    status: "available",
    method: "selected_route_fee_free_v1",
    generation: m.generation,
    observed_at_ms: m.observed_at_ms,
    expires_at_ms: m.expires_at_ms,
    input_amount: m.input_amount,
    policy: m.policy,
    max_spread: m.max_spread,
    router_operations: [op],
    stages: [
      {
        reserve_in: "10000",
        reserve_out: "10000",
        input_amount: "100",
        output_amount: "99",
        book_input: "0",
        max_maker_fills: 1,
        book_side: "bid",
        book: [],
      },
    ],
    baseline_amount_out: "100",
    fee_free_amount_out: "99",
    order_movement: "1",
    trading_fee: "1",
    tax_hook_cost: "0",
    wrapper_cost: "0",
    reference_divergence: "20",
  };
  return {
    body: {
      token_in: "in",
      token_out: "out",
      estimated_amount_out: "98",
      estimated_amount_out_net: "98",
      spot_amount_out: "120",
      execution_metadata: m,
      router_operations: [op],
      economic_attribution: a,
      order_economics: d,
    },
    params: {
      token_in: "in",
      token_out: "out",
      amount_in: "100",
      max_spread: "0.05",
    },
  };
}
test("independent finite-size CP fixture separates1% movement and1% fee from20raw reference divergence", () => {
  const f = fixture(),
    r = validate(f.body, f.params, 2000);
  assert.equal(r.slippage, 1);
  assert.equal(r.fee, 1);
  assert.equal(r.output, "98");
  assert.equal(r.metrics, true);
});
test("diagnostic-only same-ratio reserve forgery cannot change finite-size slippage", () => {
  const f = fixture(),
    d = f.body.order_economics;
  d.stages[0].reserve_in = d.stages[0].reserve_out = "5000";
  d.stages[0].output_amount = d.fee_free_amount_out = "98";
  d.order_movement = "2";
  assert.throws(() => validate(f.body, f.params, 2000), /reserve_binding/);
});
test("a forged original pool boundary fails even if original and advisory reserves are both rewritten", () => {
  const f = fixture(),
    h = f.body.execution_metadata.hops[0],
    s = f.body.order_economics.stages[0];
  h.reserve_in = h.reserve_out = s.reserve_in = s.reserve_out = "5000";
  assert.throws(() => validate(f.body, f.params, 2000), /original_stage/);
});
test("original expiry and actor-bearing response cannot be counted as useful quote", () => {
  const f = fixture();
  assert.throws(() => validate(f.body, f.params, 21000), /original_window/);
  f.body.execution_metadata.trader = "synthetic-private-actor";
  assert.throws(() => validate(f.body, f.params, 2000), /execution_binding/);
});
test("unavailable advisory diagnostics stay separate from valid executable output", () => {
  const f = fixture();
  f.body.order_economics = { version: 1, status: "unavailable" };
  const r = validate(f.body, f.params, 2000);
  assert.equal(r.useful, true);
  assert.equal(r.metrics, false);
  assert.equal(r.slippage, undefined);
});
test("contradictory execution support flags do not establish useful output", () => {
  const f = fixture();
  f.body.execution_metadata.execution_supported = true;
  f.body.execution_supported = false;
  assert.throws(() => validate(f.body, f.params, 2000), /execution_binding/);
});
test("unused-maker diagnostic mutation cannot forge0% into-50% after removing sell funding loss", () => {
  // Independent original boundaries: gross200 -> economic100 -> maker one100
  // gross output, less1commission -> net99. Fee-free200 reaches two1x makers.
  const f = fixture(),
    m = f.body.execution_metadata,
    h = m.hops[0],
    d = f.body.order_economics;
  f.params.amount_in = m.input_amount = d.input_amount = "200";
  f.body.spot_amount_out = f.body.economic_attribution.fair_amount_out = "200";
  f.body.estimated_amount_out =
    f.body.estimated_amount_out_net =
    f.body.economic_attribution.net_amount_out =
      "99";
  m.minimum_receive =
    h.min_return =
    f.body.router_operations[0].terra_swap.min_return =
      "95";
  m.policy = d.policy = "declared_hybrid_hop0";
  m.execution_market_spread = "0.052631578947368422";
  Object.assign(h, {
    offer_amount: "200",
    economic_offer_amount: "100",
    pool_input_actual: "0",
    book_offer_consumed: "100",
    pool_return_amount: "0",
    commission_amount: "0",
    book_return_net: "99",
    book_commission_amount: "1",
    spread_amount: "0",
    return_amount: "99",
    declared_hybrid: {
      pool_input: "0",
      book_input: "100",
      max_maker_fills: 2,
      book_start_hint: null,
    },
    book_terms: [
      { remaining: "100", price_atomics: "1000000000000000000" },
      { remaining: "100", price_atomics: "1000000000000000000" },
    ],
  });
  f.body.router_operations[0].terra_swap.hybrid = h.declared_hybrid;
  f.body.economic_attribution.components = {
    wrapper_loss: "0",
    trading_cost: "101",
    market_movement: "0",
    reference_divergence: "0",
  };
  d.baseline_amount_out = d.fee_free_amount_out = "200";
  d.order_movement = "0";
  d.tax_hook_cost = "100";
  d.reference_divergence = "0";
  Object.assign(d.stages[0], {
    input_amount: "200",
    output_amount: "200",
    book_input: "200",
    max_maker_fills: 2,
    book: structuredClone(h.book_terms),
  });
  assert.equal(validate(f.body, f.params, 2000).slippage, 0);
  const originalMetadata = JSON.stringify(m);
  // Only advisory unused-maker escrow/price and internally consistent derived
  // output/movement change; actual budget100 still consumes the first maker.
  d.stages[0].book[1] = {
    remaining: "200",
    price_atomics: "2000000000000000000",
  };
  d.stages[0].output_amount = d.fee_free_amount_out = "300";
  d.order_movement = "-100";
  assert.throws(() => validate(f.body, f.params, 2000), /book_binding/);
  assert.equal(JSON.stringify(m), originalMetadata);
});
test("original maker-term history is required and capped at500 rows", () => {
  const f = fixture();
  delete f.body.execution_metadata.hops[0].book_terms;
  assert.throws(() => validate(f.body, f.params, 2000), /book_binding/);
  f.body.execution_metadata.hops[0].book_terms = Array(501).fill({
    remaining: "1",
    price_atomics: "1000000000000000000",
  });
  f.body.order_economics.stages[0].book = structuredClone(
    f.body.execution_metadata.hops[0].book_terms,
  );
  assert.throws(() => validate(f.body, f.params, 2000), /book_binding/);
});
