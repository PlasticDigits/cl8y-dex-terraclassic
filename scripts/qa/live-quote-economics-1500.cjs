// Independent bounded raw-stage validator for anonymous live receipts. No production imports.
const MAX = (1n << 128n) - 1n,
  S = 10n ** 18n;
function integer(v, signed = false) {
  if (
    typeof v !== "string" ||
    !(signed ? /^(0|-?[1-9]\d{0,38})$/ : /^(0|[1-9]\d{0,38})$/).test(v)
  )
    throw Error("raw_shape");
  const n = BigInt(v);
  if (n > MAX || n < (signed ? -MAX : 0n)) throw Error("raw_bound");
  return n;
}
function decimal(v) {
  if (typeof v !== "string" || !/^(0(?:\.\d{1,18})?|1)$/.test(v))
    throw Error("decimal_shape");
  return v === "1" ? S : BigInt((v.split(".")[1] || "").padEnd(18, "0"));
}
function book(rows, side, budget, cap) {
  if (
    !Array.isArray(rows) ||
    rows.length > 500 ||
    !["bid", "ask"].includes(side) ||
    !Number.isInteger(cap) ||
    cap < 1 ||
    cap > 100
  )
    throw Error("book_bound");
  let left = budget,
    out = 0n,
    fills = 0;
  for (const r of rows) {
    if (!r || Object.keys(r).sort().join(",") !== "price_atomics,remaining")
      throw Error("book_shape");
    const remain = integer(r.remaining),
      price = integer(r.price_atomics);
    if (!remain || !price) throw Error("book_zero");
    const inverse = (S * S) / price,
      capacity = (remain * inverse) / S;
    if (
      !inverse ||
      inverse > MAX ||
      (side === "bid" &&
        (!capacity || capacity > MAX || (capacity * price) / S > remain)) ||
      (side === "ask" && remain * price < S)
    )
      throw Error("book_terms");
    if (!left || fills >= cap) continue;
    const offered = side === "bid" ? left : (left * inverse) / S;
    const fill =
        side === "bid"
          ? offered < capacity
            ? offered
            : capacity
          : offered < remain
            ? offered
            : remain,
      cost = (fill * price) / S;
    if (!fill || !cost) continue;
    if (cost > MAX || cost > (side === "bid" ? remain : left))
      throw Error("book_cost");
    left -= side === "bid" ? fill : cost;
    out += side === "bid" ? cost : fill;
    if (out > MAX) throw Error("book_output");
    fills++;
  }
  return { consumed: budget - left, output: out };
}
/** Bounded public-clock diagnostics. Never copy raw metadata, generation, actor
 * or upstream text into a failed receipt. Keep this aligned with validation. */
function originalWindowDiagnostics(metadata, now) {
  const observed = Number.isSafeInteger(metadata?.observed_at_ms)
      ? metadata.observed_at_ms
      : null,
    expires = Number.isSafeInteger(metadata?.expires_at_ms)
      ? metadata.expires_at_ms
      : null;
  const reason =
    observed == null || expires == null
      ? "invalid_clock"
      : expires <= observed
        ? "nonpositive_window"
        : observed > now
          ? "future_observation"
          : expires <= now
            ? "expired_at_response"
            : expires - observed > 20000
              ? "overlong_window"
              : typeof metadata.generation !== "string" ||
                  !/^[a-zA-Z0-9:_-]{1,256}$/.test(metadata.generation)
                ? "invalid_generation"
                : null;
  return {
    reason,
    observed_at_ms: observed,
    expires_at_ms: expires,
    response_epoch_ms: Number.isSafeInteger(now) ? now : null,
  };
}
function validate(body, params, now) {
  const m = body?.execution_metadata,
    ops = body?.router_operations;
  if (
    !m ||
    m.version !== 1 ||
    m.protection_policy !== "quote_relative_v1" ||
    m.trader != null ||
    m.sender != null ||
    m.input_amount !== params.amount_in ||
    body.token_in !== params.token_in ||
    body.token_out !== params.token_out ||
    m.max_spread !== params.max_spread ||
    (Object.hasOwn(m, "execution_supported") &&
      typeof m.execution_supported !== "boolean") ||
    (Object.hasOwn(body, "execution_supported") &&
      (typeof body.execution_supported !== "boolean" ||
        body.execution_supported !== (m.execution_supported ?? true))) ||
    m.execution_supported === false ||
    body.execution_blocker != null ||
    !Array.isArray(m.hops) ||
    !m.hops.length ||
    m.hops.length > 4 ||
    !Array.isArray(ops) ||
    ops.length !== m.hops.length
  )
    throw Error("execution_binding");
  if (originalWindowDiagnostics(m, now).reason) throw Error("original_window");
  const net = integer(body.estimated_amount_out_net),
    fair = integer(body.spot_amount_out),
    input = integer(m.input_amount),
    u = decimal(m.max_spread);
  if (
    !net ||
    !fair ||
    !input ||
    !u ||
    u > S / 2n ||
    body.estimated_amount_out !== body.estimated_amount_out_net ||
    integer(m.minimum_receive) < (net * (S - u) + S - 1n) / S ||
    integer(m.minimum_receive) > net ||
    (fair > net ? fair - net : net - fair) * 100n >=
      (fair > net ? fair : net) * 99n
  )
    throw Error("output_floor");
  let ceiling = 0n,
    prior = m.wrapper ? integer(m.wrapper.route_input_amount) : input;
  for (let i = 0; i < m.hops.length; i++) {
    const h = m.hops[i],
      t = ops[i]?.terra_swap;
    if (
      !t ||
      integer(h.offer_amount) !== prior ||
      t.min_return !== h.min_return
    )
      throw Error("operation_binding");
    const offer = t.offer_asset_info?.token?.contract_addr,
      ask = t.ask_asset_info?.token?.contract_addr;
    if (
      offer !== h.offer_token ||
      ask !== h.ask_token ||
      (i && h.offer_token !== m.hops[i - 1].ask_token)
    )
      throw Error("operation_path");
    const output = integer(h.return_amount),
      floor = integer(h.min_return),
      pool = integer(h.pool_return_amount),
      fee = integer(h.commission_amount),
      bn = integer(h.book_return_net),
      P = integer(h.pool_input_actual),
      C = integer(h.book_offer_consumed),
      spread = integer(h.spread_amount);
    if (!output || floor < (output * (S - u) + S - 1n) / S || floor > output)
      throw Error("hop_floor");
    const D = pool + fee + bn,
      expected = P && C ? (pool * C) / P : 0n,
      N =
        (spread < pool + fee ? spread : pool + fee) +
        (expected > bn ? expected - bn : 0n);
    if (!D || D > MAX || N > D) throw Error("market_spread");
    const num = (N * S + D * u) * S,
      den = D * (S - u),
      derived = (num + den - 1n) / den;
    ceiling =
      ceiling > (derived > S ? S : derived)
        ? ceiling
        : derived > S
          ? S
          : derived;
    prior = output;
  }
  if (decimal(m.execution_market_spread) !== ceiling)
    throw Error("market_ceiling");
  if (m.wrapper) {
    const w = m.wrapper,
      wrap = ["uluna", "uusd"].includes(params.token_in),
      unwrap = ["uluna", "uusd"].includes(params.token_out);
    if (
      w.input_token !== params.token_in ||
      w.output_token !== params.token_out ||
      w.input_amount !== m.input_amount ||
      w.wrap_input !== wrap ||
      w.unwrap_output !== unwrap ||
      integer(w.route_input_amount) > input ||
      (!wrap && integer(w.route_input_amount) !== input) ||
      integer(w.route_output_amount) !== prior ||
      net > prior ||
      (!unwrap && net !== prior) ||
      integer(w.minimum_receive_base) < integer(m.minimum_receive)
    )
      throw Error("wrapper_binding");
  } else if (prior !== net) throw Error("operation_output");
  const a = body.economic_attribution,
    d = body.order_economics;
  if (!a || a.status !== "available" || !d || d.status !== "available")
    return {
      useful: true,
      metrics: false,
      output: net.toString(),
      observed: m.observed_at_ms,
      expires: m.expires_at_ms,
    };
  for (const v of [a, d])
    if (
      v.version !== 1 ||
      v.generation !== m.generation ||
      v.observed_at_ms !== m.observed_at_ms ||
      v.expires_at_ms !== m.expires_at_ms
    )
      throw Error("diagnostic_cohort");
  if (
    a.method !== "spot_bridge_v1" ||
    d.method !== "selected_route_fee_free_v1" ||
    a.fair_amount_out !== body.spot_amount_out ||
    a.net_amount_out !== body.estimated_amount_out_net ||
    d.input_amount !== m.input_amount ||
    d.policy !== m.policy ||
    d.max_spread !== m.max_spread ||
    JSON.stringify(d.router_operations) !== JSON.stringify(ops) ||
    !Array.isArray(d.stages) ||
    d.stages.length !== m.hops.length
  )
    throw Error("diagnostic_binding");
  const c = a.components,
    loss = integer(c.wrapper_loss, true),
    trade = integer(c.trading_cost, true),
    movement = integer(c.market_movement, true),
    reference = integer(c.reference_divergence, true);
  if (loss + trade + movement + reference !== fair - net)
    throw Error("attribution_sum");
  let spot = input,
    free = input,
    fees = 0n;
  const project = (x, start) => {
    for (let j = start; j < d.stages.length; j++) {
      x =
        (x * integer(d.stages[j].reserve_out)) /
        integer(d.stages[j].reserve_in);
      if (x > MAX) throw Error("projection_overflow");
    }
    return x;
  };
  for (let i = 0; i < d.stages.length; i++) {
    const s = d.stages[i],
      h = m.hops[i],
      rin = integer(s.reserve_in),
      rout = integer(s.reserve_out);
    if (
      !rin ||
      !rout ||
      s.reserve_in !== h.reserve_in ||
      s.reserve_out !== h.reserve_out ||
      s.book_side !== h.book_side ||
      integer(s.input_amount) !== free
    )
      throw Error("reserve_binding");
    const economic = integer(h.economic_offer_amount ?? h.offer_amount),
      budget = h.declared_hybrid ? integer(h.declared_hybrid.book_input) : 0n,
      cap = h.declared_hybrid?.max_maker_fills ?? 1,
      freeBudget = (free * budget) / economic;
    if (integer(s.book_input) !== freeBudget || s.max_maker_fills !== cap)
      throw Error("counterfactual_budget");
    // Removing charges can reach a maker unused by the actual smaller budget.
    // Bind every original reachable term before either walk, not only actual fills.
    if (
      !Array.isArray(h.book_terms) ||
      h.book_terms.length > 500 ||
      !Array.isArray(s.book) ||
      s.book.length > 500 ||
      JSON.stringify(s.book) !== JSON.stringify(h.book_terms)
    )
      throw Error("book_binding");
    const B = book(s.book, s.book_side, budget, cap),
      F = book(s.book, s.book_side, freeBudget, cap),
      P = integer(h.pool_input_actual),
      pool = integer(h.pool_return_amount),
      pc = integer(h.commission_amount),
      bn = integer(h.book_return_net),
      bc = integer(h.book_commission_amount);
    if (
      B.consumed !== integer(h.book_offer_consumed) ||
      B.output !== bn + bc ||
      B.consumed + P !== economic ||
      (P * rout) / (rin + P) !== pool + pc ||
      (P * rout) / rin - (pool + pc) !== integer(h.spread_amount)
    )
      throw Error("original_stage");
    const freePool = free - F.consumed;
    if (rin + freePool > MAX) throw Error("pool_overflow");
    free = F.output + (freePool * rout) / (rin + freePool);
    spot = (spot * rout) / rin;
    if (free !== integer(s.output_amount) || free > MAX || spot > MAX)
      throw Error("counterfactual_output");
    fees += project(pool + bn + pc + bc, i + 1) - project(pool + bn, i + 1);
  }
  const baseline = integer(d.baseline_amount_out),
    out = integer(d.fee_free_amount_out),
    order = integer(d.order_movement, true);
  if (
    !baseline ||
    !out ||
    spot !== baseline ||
    free !== out ||
    baseline - out !== order ||
    fair - reference !== baseline ||
    integer(d.trading_fee) !== fees ||
    integer(d.tax_hook_cost) + fees !== trade ||
    integer(d.wrapper_cost) !== loss ||
    integer(d.reference_divergence, true) !== reference
  )
    throw Error("diagnostic_sum");
  return {
    useful: true,
    metrics: true,
    output: net.toString(),
    observed: m.observed_at_ms,
    expires: m.expires_at_ms,
    slippage: (Number(order) / Number(baseline)) * 100,
    fee: (Number(fees) / Number(baseline)) * 100,
  };
}
module.exports = { integer, book, validate, originalWindowDiagnostics };
