#!/usr/bin/env node
// Actual shared validator/measurement and product explanation under a controlled DOM clock.
const assert = require("node:assert/strict");
const { test } = require("node:test");
const path = require("node:path");
const { createRequire } = require("node:module");
const frontend = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
);
const { JSDOM } = frontend("jsdom");
const {
  sharedSources,
  checkAttestation,
  installRuntime,
  measurementSource,
  browserSampleFailed,
  COPIES,
} = require("./browser-route-attribution-1483.cjs");
const shared = sharedSources();
const epoch = 1700000000000;
const sha = "a".repeat(40);
function attestation() {
  return {
    overall: "PASS",
    expected_revision: sha,
    frontend: {
      source_sha: sha,
      deployment_status: "finished",
      artifact: {
        html_sha256: "b".repeat(64),
        root_scripts: [{ asset: "index.js", sha256: "c".repeat(64) }],
      },
    },
    backend: { git_sha: sha },
    backend_deployment: { source_sha: sha, deployment_status: "finished" },
  };
}
function quote() {
  const now = epoch;
  return {
    token_in: "in",
    token_out: "out",
    estimated_amount_out_net: "900000",
    estimated_amount_out: "900000",
    spot_amount_out: "1000000",
    execution_metadata: {
      protection_policy: "quote_relative_v1",
      max_spread: "0.05",
      input_amount: "1000000",
      minimum_receive: "855000",
      observed_at_ms: now - 1000,
      expires_at_ms: now + 19000,
      generation: "original",
      execution_market_spread: "0.052631578947368422",
      hops: [
        {
          pool_return_amount: "900000",
          commission_amount: "0",
          book_return_net: "0",
          pool_input_actual: "1000000",
          book_offer_consumed: "0",
          spread_amount: "0",
          return_amount: "900000",
          min_return: "855000",
        },
      ],
    },
    router_operations: [{ terra_swap: { min_return: "855000" } }],
    economic_attribution: {
      version: 1,
      status: "available",
      method: "spot_bridge_v1",
      generation: "original",
      observed_at_ms: now - 1000,
      expires_at_ms: now + 19000,
      fair_amount_out: "1000000",
      net_amount_out: "900000",
      components: {
        wrapper_loss: "0",
        trading_cost: "0",
        market_movement: "0",
        reference_divergence: "100000",
      },
      dominant_cause: "reference_divergence",
    },
  };
}
function fixture({ entry = "swap", offscreen = false } = {}) {
  const payId =
    entry === "trade"
      ? "limit-order-escrow-amount-input"
      : "swap-you-pay-amount";
  const receiveId =
    entry === "trade" ? "trade-market-expected-receive" : "swap-you-receive";
  const prefix = entry === "trade" ? "trade-market" : "swap";
  const dom = new JSDOM(
    `<input data-testid="${payId}">${entry === "trade" ? `<span data-testid="${receiveId}"></span>` : `<input data-testid="${receiveId}">`}<p data-testid="${prefix}-impact-aside"></p>`,
    { runScripts: "outside-only", url: "https://dex.cl8y.com" },
  );
  const w = dom.window;
  let now = 0,
    next = 1;
  const frames = new Map(),
    intervals = new Map();
  Object.defineProperty(w.performance, "now", { value: () => now });
  w.Date.now = () => epoch + now;
  Object.defineProperty(w, "innerWidth", { value: 1440 });
  Object.defineProperty(w, "innerHeight", { value: 1000 });
  w.fetch = async () => {};
  w.__prohibited1483 = async () => {};
  w.__writes1473 = 0;
  w.requestAnimationFrame = (fn) => {
    const id = next++;
    frames.set(id, fn);
    return id;
  };
  w.cancelAnimationFrame = (id) => frames.delete(id);
  w.setInterval = (fn) => {
    const id = next++;
    intervals.set(id, fn);
    return id;
  };
  w.clearInterval = (id) => intervals.delete(id);
  w.eval(
    `window.__validate1483 = ${shared.validator}; window.__explain1483 = ${shared.explanation}; window.__precise1483 = ${shared.precision}; (${installRuntime.toString()})(${JSON.stringify({ copies: COPIES })});`,
  );
  const pay = w.document.querySelector(`[data-testid="${payId}"]`),
    receive = w.document.querySelector(`[data-testid="${receiveId}"]`),
    aside = w.document.querySelector(`[data-testid="${prefix}-impact-aside"]`);
  for (const [node, top] of [
    [pay, 100],
    [receive, 200],
    [aside, offscreen ? 1100 : 300],
  ]) {
    node.style.opacity = "1";
    node.getBoundingClientRect = () => ({
      width: 400,
      height: 32,
      left: 100,
      right: 500,
      top,
      bottom: top + 32,
    });
  }
  const expected = {
    input: "in",
    output: "out",
    amount: "1000000",
    spread: "0.05",
    decimals: 6,
    entry,
  };
  w.eval(`(${measurementSource(shared, entry)})`)(expected);
  pay.value = "1";
  pay.dispatchEvent(new w.Event("input", { bubbles: true }));
  const q = quote();
  const row = {
    started: 1,
    ended: 300,
    status: 200,
    params: {
      token_in: "in",
      token_out: "out",
      amount_in: "1000000",
      max_spread: "0.05",
      protection_policy: "quote_relative_v1",
    },
    classified: true,
    useful: true,
    output: "900000",
    minimum: "855000",
    observed: epoch - 1000,
    expires: epoch + 19000,
    diagnostic: {
      valid: true,
      percent: "10.00",
      explanation: w.__explain1483(q),
    },
  };
  w.__quotes1473.push(row);
  if (entry === "trade") receive.textContent = "0.9 UST1";
  else receive.value = "0.9";
  aside.textContent =
    "Fair-reference deviation 10.00%. Reference and pool prices differ most.";
  return {
    w,
    row,
    aside,
    receive,
    q,
    expected,
    tick(at) {
      now = at;
      [...intervals.values()].forEach((fn) => fn());
    },
    frame(at) {
      now = at;
      const all = [...frames.values()];
      frames.clear();
      all.forEach((fn) => fn(at));
    },
    result() {
      return JSON.parse(JSON.stringify(w.__measurement1473.read()));
    },
    close() {
      w.close();
    },
  };
}

test("independent deployment/source/root hashes are mandatory", () => {
  assert.ok(checkAttestation(attestation(), sha));
  for (const mutation of [
    (a) => {
      a.overall = "FAIL";
    },
    (a) => {
      a.frontend.source_sha = "d".repeat(40);
    },
    (a) => {
      a.frontend.artifact.root_scripts[0].asset = "../private";
    },
    (a) => {
      a.backend_deployment.deployment_status = "running";
    },
  ]) {
    const a = attestation();
    mutation(a);
    assert.throws(() => checkAttestation(a, sha));
  }
});
test("reuses actual validator and rejects original expiry, actors and lowered floors", () => {
  const f = fixture();
  const params = f.row.params;
  assert.equal(f.w.__validate1483(f.q, params, epoch), true);
  for (const mutate of [
    (q) => {
      q.execution_metadata.expires_at_ms = epoch;
    },
    (q) => {
      q.execution_metadata.trader = "private";
    },
    (q) => {
      q.execution_metadata.minimum_receive = "1";
    },
  ]) {
    const q = structuredClone(f.q);
    mutate(q);
    assert.equal(f.w.__validate1483(q, params, epoch), false);
  }
  f.close();
});
for (const entry of ["swap", "trade"]) {
  test(`${entry}: original input and two visible frames include exact allowed explanation`, () => {
    const f = fixture({ entry });
    f.tick(300);
    f.frame(320);
    assert.equal(f.w.__measurement1473.done, null);
    f.frame(340);
    const r = f.result();
    assert.equal(r.useful, true);
    assert.equal(r.input_to_dom_ms, 340);
    assert.equal(r.policy_opt_in, true);
    f.close();
  });
  test(`${entry}: missing, injected, duplicated and offscreen copy cannot become useful`, () => {
    for (const text of [
      "Cost breakdown unavailable.",
      "<script>private</script>",
      "Fair-reference deviation 10.00%. Reference and pool prices differ most. 2%",
    ]) {
      const f = fixture({ entry });
      f.aside.textContent = text;
      f.tick(300);
      f.frame(320);
      f.frame(340);
      assert.equal(f.result().useful, false);
      f.close();
    }
    const f = fixture({ entry, offscreen: true });
    f.tick(300);
    f.frame(320);
    f.frame(340);
    assert.equal(f.result().useful, false);
    f.close();
  });
  test(`${entry}: change or expiry between frames never certifies stale output`, () => {
    const f = fixture({ entry });
    f.tick(300);
    f.frame(320);
    f.row.expires = epoch + 330;
    f.frame(340);
    assert.equal(f.result().useful, false);
    f.close();
  });
}
test("all public copies are fixed and carry no second percentage", () => {
  assert.ok(COPIES.every((copy) => !copy.includes("%") && copy.length < 100));
});

test("750ms is applied to unrounded original-event useful timing, never an error or absent result", () => {
  const row = {
    useful: true,
    classified: true,
    policy_opt_in: true,
    input_to_dom_ms: 750,
  };
  assert.equal(browserSampleFailed(row), false);
  assert.equal(browserSampleFailed({ ...row, input_to_dom_ms: 750.001 }), true);
  assert.equal(
    browserSampleFailed({ ...row, useful: false, input_to_dom_ms: 100 }),
    true,
  );
  assert.equal(browserSampleFailed({ ...row, input_to_dom_ms: null }), true);
  assert.equal(browserSampleFailed({ ...row, input_to_dom_ms: NaN }), true);
  assert.equal(browserSampleFailed({ ...row, input_to_dom_ms: -1 }), true);
});
test("ancestral overflow clipping is excluded from useful explanation visibility", () => {
  const f = fixture();
  const parent = f.w.document.createElement("div");
  parent.style.overflowY = "hidden";
  parent.getBoundingClientRect = () => ({
    top: 0,
    bottom: 200,
    left: 0,
    right: 1000,
  });
  f.aside.parentNode.insertBefore(parent, f.aside);
  parent.appendChild(f.aside);
  f.tick(300);
  f.frame(320);
  f.frame(340);
  assert.equal(f.result().useful, false);
  f.close();
});
