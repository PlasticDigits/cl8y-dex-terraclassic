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
const { eligibleQuotesAtSnapshot } = require("./live-input-clock-1499.cjs");
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
