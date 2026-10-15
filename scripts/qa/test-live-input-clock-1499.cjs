const { test } = require("node:test"),
  assert = require("node:assert/strict");
const path = require("node:path"),
  { createRequire } = require("node:module");
const rf = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
);
const { JSDOM } = rf("jsdom");
const { installInputClock } = require("./live-input-clock-1499.cjs");
function harness() {
  const dom = new JSDOM(
    '<input data-testid="swap-you-pay-amount"/><input data-testid="swap-you-receive"/>',
    { runScripts: "outside-only" },
  );
  const w = dom.window;
  let now = 0,
    epoch = 1800000000000,
    id = 0;
  Object.defineProperty(w.document, "visibilityState", {
    value: "visible",
    configurable: true,
  });
  Object.defineProperty(w.document, "hidden", {
    value: false,
    configurable: true,
  });
  Object.defineProperty(w.performance, "now", { value: () => now });
  w.Date.now = () => epoch + now;
  const frames = new Map();
  w.requestAnimationFrame = (cb) => {
    frames.set(++id, cb);
    return id;
  };
  w.cancelAnimationFrame = (key) => frames.delete(key);
  for (const n of w.document.querySelectorAll("input")) {
    n.style.opacity = "1";
    n.getBoundingClientRect = () => ({
      width: 200,
      height: 40,
      left: 10,
      right: 210,
      top: 10,
      bottom: 50,
    });
  }
  w.eval("(" + installInputClock.toString() + ")()");
  const pay = w.document.querySelector('[data-testid="swap-you-pay-amount"]'),
    receive = w.document.querySelector('[data-testid="swap-you-receive"]');
  pay.value = "132";
  pay.dispatchEvent(new w.Event("input", { bubbles: true }));
  const tick = (at) => {
    now = at;
    const callbacks = [...frames.values()];
    frames.clear();
    callbacks.forEach((cb) => cb(at));
  };
  return { w, receive, tick, close: () => dom.window.close() };
}
test("in-page two-frame result preserves original input clock regardless of later Node sampling", () => {
  const h = harness();
  h.tick(300);
  h.receive.value = "1.092895860732416547";
  h.tick(400);
  h.tick(416);
  h.tick(432);
  const first = h.w.__liveClock1499.candidates[0];
  assert.equal(first.first_dom_ms, 400);
  assert.equal(first.input_to_paint_ms, 432);
  h.tick(3522);
  assert.equal(h.w.__liveClock1499.candidates[0].input_to_paint_ms, 432);
  assert.equal(h.w.__liveClock1499.started, 0);
  h.close();
});
test("hidden/background DOM cannot become a useful painted sample", () => {
  const h = harness();
  Object.defineProperty(h.w.document, "hidden", {
    value: true,
    configurable: true,
  });
  Object.defineProperty(h.w.document, "visibilityState", {
    value: "hidden",
    configurable: true,
  });
  h.receive.value = "1";
  h.tick(400);
  h.tick(416);
  h.tick(432);
  assert.equal(h.w.__liveClock1499.candidates.length, 0);
  Object.defineProperty(h.w.document, "hidden", {
    value: false,
    configurable: true,
  });
  Object.defineProperty(h.w.document, "visibilityState", {
    value: "visible",
    configurable: true,
  });
  h.tick(1000);
  h.tick(1016);
  h.tick(1032);
  assert.equal(h.w.__liveClock1499.candidates[0].input_to_paint_ms, 1032);
  assert.equal(h.w.__liveClock1499.started, 0);
  h.close();
});
test("blank then identical output retries an aborted paint without resetting the input or earliest DOM clock", () => {
  const h = harness();
  h.receive.value = "1";
  h.tick(400);
  h.tick(416);
  h.receive.value = "";
  h.tick(432);
  assert.equal(h.w.__liveClock1499.candidates.length, 0);
  h.receive.value = "1";
  h.tick(448);
  h.tick(464);
  h.tick(480);
  assert.equal(h.w.__liveClock1499.dom_observations[0].at_ms, 400);
  assert.equal(h.w.__liveClock1499.candidates[0].first_dom_ms, 448);
  assert.equal(h.w.__liveClock1499.candidates[0].input_to_paint_ms, 480);
  assert.equal(h.w.__liveClock1499.started, 0);
  h.close();
});
test("a visibility-aborted identical-value paint can qualify after foreground returns on the original clock", () => {
  const h = harness();
  h.receive.value = "1";
  h.tick(400);
  Object.defineProperty(h.w.document, "hidden", {
    value: true,
    configurable: true,
  });
  h.tick(416);
  Object.defineProperty(h.w.document, "hidden", {
    value: false,
    configurable: true,
  });
  h.tick(432);
  h.tick(448);
  h.tick(464);
  assert.equal(h.w.__liveClock1499.candidates[0].input_to_paint_ms, 464);
  assert.equal(h.w.__liveClock1499.dom_observations[0].at_ms, 400);
  assert.equal(h.w.__liveClock1499.started, 0);
  h.close();
});
const { eligibleQuotesAtSnapshot } = require("./live-input-clock-1499.cjs");
const { displayedQuoteRetention } = require("./live-input-clock-1499.cjs");
const {
  updateDisplayedQuoteOwners,
  matchingDisplayedQuotes,
} = require("./live-input-clock-1499.cjs");
test("unchanged display then blank preserves the expired possible owner through the actual update sequence", () => {
  const old = {
    status: 200,
    useful: true,
    output: "1",
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const fresh = { ...old, observed: 11000, expires: 31000, absolute_at: 12000 };
  const events = [old, fresh];
  let state = updateDisplayedQuoteOwners(
    { signature: null, owners: [] },
    "same",
    eligibleQuotesAtSnapshot(events, 20000),
  );
  state = updateDisplayedQuoteOwners(
    state,
    "same",
    eligibleQuotesAtSnapshot(events, 21001),
  );
  assert.equal(state.owners.length, 2);
  state = updateDisplayedQuoteOwners(state, null, []);
  assert.equal(
    displayedQuoteRetention(state.owners, events, 21002, state.unavailable),
    "ambiguous",
  );
  state = updateDisplayedQuoteOwners(state, "same", [fresh]);
  assert.equal(state.owners.length, 1);
  assert.equal(
    displayedQuoteRetention(state.owners, events, 21003),
    "unexpired",
  );
});
test("same output with different visible metrics matches the compatible newer quote and establishes a distinguishable transition", () => {
  const old = {
    output: "1",
    metrics: true,
    observed: 1000,
    expires: 21000,
    slippage: 1,
    fee: 1,
  };
  const fresh = {
    ...old,
    observed: 11000,
    expires: 31000,
    slippage: 2,
    fee: 3,
  };
  const display = { value: "1", slippage: "Slippage 2%", fee: "Fee 3%" };
  const matches = matchingDisplayedQuotes(
    [old, fresh],
    display,
    String,
    (n) => n + "%",
  );
  assert.deepEqual(matches, [fresh]);
  let state = updateDisplayedQuoteOwners(
    { signature: null, owners: [] },
    "old-metrics",
    [old],
  );
  state = updateDisplayedQuoteOwners(state, "new-metrics", matches);
  assert.deepEqual(state.owners, [fresh]);
  assert.equal(
    matchingDisplayedQuotes([old], display, String, (n) => n + "%").length,
    0,
  );
});
test("owner union overflow explicitly loses proof and cannot discard expired owners to claim adoption", () => {
  const owners = Array.from({ length: 600 }, (_, i) => ({
    observed: i + 1,
    expires: 1000 + i,
    output: "1",
  }));
  const state = updateDisplayedQuoteOwners(
    { signature: "same", owners },
    "same",
    [{ observed: 700, expires: 31000, output: "1" }],
  );
  assert.equal(state.unavailable, true);
  assert.equal(
    displayedQuoteRetention(state.owners, [], 21001, state.unavailable),
    "unavailable",
  );
  assert.equal(
    updateDisplayedQuoteOwners(state, "same", [
      { observed: 701, expires: 31001, output: "1" },
    ]).unavailable,
    true,
  );
});
test("unknown advisory metrics cannot validate invented zero in unknown-only or mixed same-output cohorts", () => {
  const unknown = { output: "1", metrics: false };
  const known = { output: "1", metrics: true, slippage: 2, fee: 3 };
  const zero = { value: "1", slippage: "Slippage 0%", fee: "Fee 0%" };
  const missing = {
    value: "1",
    slippage: "Slippage Unavailable",
    fee: "Fee Unavailable",
  };
  const format = (n) => n + "%";
  assert.equal(
    matchingDisplayedQuotes([unknown], zero, String, format).length,
    0,
  );
  assert.equal(
    matchingDisplayedQuotes([unknown, known], zero, String, format).length,
    0,
  );
  assert.deepEqual(
    matchingDisplayedQuotes([unknown, known], missing, String, format),
    [unknown],
  );
  assert.deepEqual(
    matchingDisplayedQuotes(
      [unknown, known],
      { value: "1", slippage: "Slippage 2%", fee: "Fee 3%" },
      String,
      format,
    ),
    [known],
  );
  assert.equal(
    matchingDisplayedQuotes(
      [unknown],
      { ...missing, fee: "Fee 0%" },
      String,
      format,
    ).length,
    0,
  );
});
test("an expired displayed improvement is not revived by an unused lower-output quote", () => {
  const improved = {
    status: 200,
    useful: true,
    output: "138",
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const unused = {
    status: 200,
    useful: true,
    output: "137",
    observed: 11000,
    expires: 31000,
    absolute_at: 12000,
  };
  assert.equal(eligibleQuotesAtSnapshot([improved, unused], 21075).length, 1);
  assert.equal(
    displayedQuoteRetention([improved], [improved, unused], 21075),
    "expired",
  );
});
test("a uniquely observed unexpired displayed quote still requires retention", () => {
  const quote = {
    status: 200,
    useful: true,
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  assert.equal(displayedQuoteRetention([quote], [quote], 20999), "unexpired");
  assert.equal(displayedQuoteRetention([quote], [quote], 21000), "expired");
});
test("indistinguishable cohorts require qualified ownership rather than fabricated renewal", () => {
  const old = {
    status: 200,
    useful: true,
    output: "1",
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const fresh = { ...old, observed: 11000, expires: 31000, absolute_at: 12000 };
  assert.equal(
    displayedQuoteRetention([old, fresh], [old, fresh], 21001),
    "ambiguous",
  );
  assert.equal(
    displayedQuoteRetention([old, fresh], [old, fresh], 20999),
    "unexpired",
  );
  assert.equal(displayedQuoteRetention([], [old, fresh], 20999), "unobserved");
});
test("a current typed negative revokes displayed retention at its original capture clock", () => {
  const quote = {
    status: 200,
    useful: true,
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const negative = { status: 422, absolute_at: 5005 };
  assert.equal(
    displayedQuoteRetention([quote], [quote, negative], 5000),
    "unexpired",
  );
  assert.equal(
    displayedQuoteRetention([quote], [quote, negative], 5005),
    "revoked",
  );
});
test("a DOM captured1ms before expiry stays valid when CDP delivers after expiry", () => {
  const quote = {
    status: 200,
    useful: true,
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const sample = { sample_epoch_ms: 20999, value: "1" };
  const nodeDeliveryEpoch = 21005;
  assert.equal(quote.expires > nodeDeliveryEpoch, false);
  assert.equal(
    eligibleQuotesAtSnapshot([quote], sample.sample_epoch_ms).length,
    1,
  );
  assert.equal(eligibleQuotesAtSnapshot([quote], 21000).length, 0);
});
test("a negative response after the DOM capture cannot revoke that earlier valid sample", () => {
  const positive = {
    status: 200,
    useful: true,
    observed: 1000,
    expires: 21000,
    absolute_at: 2000,
  };
  const negative = {
    status: 422,
    reason: "unsupported_semantics",
    absolute_at: 5005,
  };
  assert.equal(eligibleQuotesAtSnapshot([positive, negative], 5000).length, 1);
  assert.equal(eligibleQuotesAtSnapshot([positive, negative], 5005).length, 0);
});
