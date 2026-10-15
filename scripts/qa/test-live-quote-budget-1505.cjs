const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const {
  CONTEXT_QUOTE_MAXIMUM,
  quoteBudgetProfile,
  contextQuoteAllowance,
  quoteBudgetExhausted,
} = require("./live-quote-budget-1505.cjs");
const { installReadGuard } = require("./live-read-guard-1499.cjs");
const { recordWindowQuoteOutcome } = require("./live-input-clock-1499.cjs");

// Exercise the unchanged pre-transport guard with mocked fetch, never HTTP or Chrome.
function guardedTransport(remaining) {
  let calls = 0;
  class XMLHttpRequest {}
  class HTMLFormElement {}
  const window = { fetch: async () => (++calls, { ok: true }) };
  vm.runInNewContext(`(${installReadGuard.toString()})(config)`, {
    config: {
      frontend: "https://dex.cl8y.com",
      backend: "https://indexer.dex.cl8y.com",
      remaining,
    },
    window,
    location: { href: "https://dex.cl8y.com/" },
    URL,
    DOMException,
    XMLHttpRequest,
    HTMLFormElement,
    navigator: {},
    document: { addEventListener() {} },
  });
  return {
    read: () => window.fetch("https://indexer.dex.cl8y.com/api/v1/route/solve/progress"),
    state: window.__liveReadGuard1499,
    calls: () => calls,
  };
}

async function replayCohort(totalBudget, requestedPerWindow) {
  // The two directions' initialized FIRST/RETEST solves spend the same global counter.
  let consumed = 4;
  const windows = [];
  for (const requested of requestedPerWindow) {
    const contextStart = consumed;
    const allowance = contextQuoteAllowance(totalBudget, consumed);
    const transport = guardedTransport(allowance);
    let admitted = 0;
    while (admitted < requested && !quoteBudgetExhausted(totalBudget, consumed, contextStart, allowance)) {
      await transport.read();
      consumed++;
      admitted++;
    }
    windows.push({
      admitted,
      complete: admitted === requested,
      transportCalls: transport.calls(),
      rejected: transport.state.rejected,
    });
  }
  return { consumed, windows };
}

test("native v9 alone receives 800 global calls; generic shard/cohort limits remain 600/1800", () => {
  assert.deepEqual(quoteBudgetProfile(false), {
    measurementVersion: "foreground_correlated_native_availability_v8",
    quoteProgressBudget: 600,
    shardCount: 3,
    cohortQuoteProgressBudget: 1800,
  });
  assert.deepEqual(quoteBudgetProfile(true), {
    measurementVersion: "foreground_correlated_native_availability_v9",
    quoteProgressBudget: 800,
    shardCount: 1,
    cohortQuoteProgressBudget: 800,
  });
  assert.equal(CONTEXT_QUOTE_MAXIMUM, 600);
  assert.ok(Object.isFrozen(quoteBudgetProfile(true)));
  assert.ok(Object.isFrozen(quoteBudgetProfile(false)));
});

test("the retained 600 allowance reproduces the fourth window's 131-call cutoff", async () => {
  // First three counts are retained observations. 179 is a bounded synthetic
  // continuation of the partial phone's 131 calls/139.626s to a full 190s.
  const run = await replayCohort(600, [152, 162, 151, 179]);
  assert.equal(run.consumed, 600);
  assert.deepEqual(run.windows.map((w) => w.admitted), [152, 162, 151, 131]);
  assert.deepEqual(run.windows.map((w) => w.complete), [true, true, true, false]);
  assert.ok(run.windows.every((w) => w.transportCalls === w.admitted && w.rejected === 0));
});

test("native 800 covers the observed workload plus bounded continuation without resetting between windows", async () => {
  const run = await replayCohort(quoteBudgetProfile(true).quoteProgressBudget, [152, 162, 151, 179]);
  assert.equal(run.consumed, 648);
  assert.ok(run.windows.every((w) => w.complete));
  assert.deepEqual(run.windows.map((w) => w.admitted), [152, 162, 151, 179]);
  assert.equal(contextQuoteAllowance(800, 469), 331);
  assert.equal(contextQuoteAllowance(800, 648), 152);
});

test("800 is a global cap including initialization; an 801st call never reaches transport", async () => {
  const first = guardedTransport(contextQuoteAllowance(800, 4));
  for (let i = 0; i < 600; i++) await first.read();
  assert.equal(first.calls(), 600);
  await assert.rejects(first.read(), /Read-only/);
  assert.equal(first.calls(), 600);
  const second = guardedTransport(contextQuoteAllowance(800, 604));
  for (let i = 0; i < 196; i++) await second.read();
  await assert.rejects(second.read(), /Read-only/);
  assert.equal(4 + first.calls() + second.calls(), 800);
  const exhausted = guardedTransport(contextQuoteAllowance(800, 800));
  await assert.rejects(exhausted.read(), /Read-only/);
  assert.equal(exhausted.calls(), 0);
});

test("a native context exhausting its unchanged 600 maximum stops even before the global 800 cap", async () => {
  const run = await replayCohort(800, [601]);
  assert.equal(run.consumed, 604);
  assert.equal(run.windows[0].complete, false);
  assert.equal(run.windows[0].transportCalls, 600);
  assert.equal(run.windows[0].rejected, 0);
  assert.equal(quoteBudgetExhausted(800, 603, 4, 600), false);
  assert.equal(quoteBudgetExhausted(800, 604, 4, 600), true);
  assert.equal(quoteBudgetExhausted(600, 600, 4, 596), true);
});

test("the shared 600 context maximum is never expanded, including smaller functional-control budgets", () => {
  for (const [budget, consumed, expected] of [
    [800, 0, 600], [800, 4, 600], [800, 200, 600],
    [800, 469, 331], [800, 800, 0], [800, 801, 0],
    [600, 4, 596], [120, 0, 120], [120, 120, 0],
  ]) assert.equal(contextQuoteAllowance(budget, consumed), expected);
  for (const [budget, consumed] of [[801, 0], [0, 0], [800, -1], [800, NaN], [600, 1.5]])
    assert.throws(() => contextQuoteAllowance(budget, consumed), /invalid_quote_budget_accounting/);
});

test("restoring measurement capacity cannot hide an active typed503 after later recovery", () => {
  const row = { responses: [], failures: [] };
  recordWindowQuoteOutcome(row, { status: 200, useful: true }, true);
  recordWindowQuoteOutcome(row, { status: 503, reason: "missing_observations", at_ms: 119836 }, true);
  recordWindowQuoteOutcome(row, { status: 200, useful: true }, true);
  assert.equal(contextQuoteAllowance(800, 600), 200);
  assert.deepEqual(row.failures, ["native_window_not_useful"]);
  assert.equal(row.quote_availability.not_useful, 1);
});

async function actualWindowScenario({ activeMs = 0, drainMs = 0, budget = true, exception = null, unavailable = false } = {}) {
  const source = fs.readFileSync(path.join(__dirname, "live-trading-lifecycle-1499.cjs"), "utf8");
  const entry = source.lastIndexOf("\nmain()\n");
  assert.ok(entry > 0);
  const listeners = new Map();
  const epoch = 100000;
  let now = epoch, started = false, allowance, sampled = 0, drainCalls = 0;
  const guard = { remaining: budget ? 0 : 599, admitted: budget ? 600 : 1,
    rejected: 0, writes: 0, providers: 0, unexpected: 0, workers: 0 };
  const request = {
    url: () => "https://indexer.dex.cl8y.com/api/v1/route/solve?token_in=syntheticfrom&token_out=syntheticto&amount_in=132000000",
    method: () => "GET",
    timing: () => ({ startTime: epoch, responseEnd: 300 }),
    failure: () => null,
  };
  const page = {
    on(name, callback) { listeners.set(name, callback); },
    async bringToFront() {},
    async evaluate(fn) {
      const text = fn.toString();
      if (text.includes("sampleEpochMs")) {
        sampled++;
        return { value: "", loading: true, unsupported: false, slippage: "", fee: "",
          details: 1, detailsOpen: true, networkFees: 1, overflow: false, geometry: [],
          read_guard: guard, sample_epoch_ms: now, visibility: "visible", hidden: false,
          clockCandidates: [], clockDomObservations: [], clockDiagnostics: {}, inputAge: now - epoch };
      }
      if (text.includes("started_epoch_ms")) return epoch;
      if (text.includes("__liveReadGuard1499")) return guard;
    },
    async waitForTimeout(ms) { now += ms; },
  };
  const context = {
    async newPage() { return page; },
    async addInitScript(_fn, config) { allowance = config.remaining; },
    async newCDPSession() { return { async send() {} }; },
    async close() {},
  };
  const fixture = {
    browser: { async newContext() { return context; } },
    setup: async () => {
      if (exception === "setup") { now += activeMs; throw Error("synthetic setup exception"); }
      return { async fill() {
        started = true;
        for (let i = 0; i < (budget ? 600 : 1); i++) listeners.get("request")(request);
        if (unavailable) listeners.get("response")({
          url: request.url, request: () => request, status: () => 503,
          headers: () => ({ "x-quote-request-id": "a".repeat(16) }),
          body: async () => Buffer.from('{"error":"missing_observations"}'),
        });
      } };
    },
    tickActive() {
      if (started) {
        now = Math.max(now, epoch + activeMs);
        if (exception === "active") throw Error("synthetic active exception");
      }
    },
  };
  const outcome = await vm.runInNewContext(source.slice(0, entry) + `
    browser = fixture.browser;
    setup = fixture.setup;
    ensure = fixture.tickActive;
    quotes = 4;
    windowCase({ label: "synthetic native", from: "syntheticfrom", to: "syntheticto", amount: "132", inDecimals: 6, outDecimals: 6, native: true }, "phone", {})
      .then(() => ({ row: receipt.windows[0], quotes }));
  `, {
    fixture,
    __dirname,
    process: { env: { DEX_LIVE_ISSUE: "1505", DEX_LIVE_SHARD: "0" }, stderr: { write() {} } },
    Date: class extends Date { static now() { return now; } },
    URL, Buffer, setTimeout, clearTimeout,
    require(name) {
      if (name === "node:module") return { createRequire: () => () => ({ chromium: {} }) };
      if (name === "./live-input-clock-1499.cjs") return {
        ...require(name),
        async drainResponseProbes() { if (drainCalls++ === 0) now += drainMs; return true; },
      };
      return require(name);
    },
  }, { timeout: 1000 });
  return { ...outcome, allowance, sampled };
}

test("actual native window stops at its context cap before sampling or sending further work", async () => {
  const r = await actualWindowScenario();
  assert.equal(r.allowance, 600);
  assert.equal(r.quotes, 604);
  assert.ok(r.row.failures.includes("quote_budget"));
  assert.equal(r.row.completed, false);
  assert.equal(r.sampled, 0);
});

test("actual driver late budget exit remains incomplete even after the old185s threshold", async () => {
  const { row } = await actualWindowScenario({ activeMs: 186000 });
  assert.equal(row.elapsed_ms, 186000);
  assert.equal(row.completed, false);
  assert.equal(row.active_window_completed, false);
  assert.equal(row.active_window_elapsed_ms, 186000);
  assert.equal(row.active_window_stop_reason, "quote_budget");
});

test("actual driver drain duration cannot complete an early budget cutoff", async () => {
  const { row } = await actualWindowScenario({ activeMs: 180000, drainMs: 5000 });
  assert.equal(row.elapsed_ms, 185000);
  assert.equal(row.completed, false);
  assert.equal(row.active_window_completed, false);
  assert.equal(row.active_window_elapsed_ms, 180000);
});

test("actual driver setup and active exceptions stay incomplete regardless of elapsed time", async () => {
  for (const exception of ["setup", "active"]) {
    const { row } = await actualWindowScenario({ activeMs: 190000, drainMs: 5000, budget: false, exception });
    assert.equal(row.completed, false);
    assert.equal(row.active_window_completed, false);
    assert.equal(row.active_window_stop_reason, exception + "_exception");
    assert.ok(row.failures.includes("incomplete_expiry_windows"));
  }
});

test("a naturally completed190s window stays completed while a real503 keeps its availability verdict FAIL", async () => {
  const { row } = await actualWindowScenario({ activeMs: 189800, budget: false, unavailable: true });
  assert.equal(row.elapsed_ms, 190000);
  assert.equal(row.completed, true);
  assert.equal(row.active_window_completed, true);
  assert.equal(row.active_window_elapsed_ms, 190000);
  assert.equal(row.active_window_stop_reason, "duration");
  assert.ok(row.failures.includes("native_window_not_useful"));
  assert.equal(row.quote_availability.not_useful, 1);
});

async function guardedDriverReceipt(native) {
  const source = fs.readFileSync(path.join(__dirname, "live-trading-lifecycle-1499.cjs"), "utf8");
  let receipt, network = 0, chrome = 0;
  const fakeFs = {
    readFileSync(filename) {
      assert.equal(filename, "/synthetic-attestation.json");
      return JSON.stringify({ overall: "FAIL" });
    },
    writeFileSync(filename, content) {
      assert.equal(filename, "/synthetic-output.json");
      receipt = JSON.parse(content);
    },
    chmodSync() {},
  };
  const fakeProcess = {
    env: {
      EXPECTED_SOURCE_SHA: "a".repeat(40),
      DEX_LIVE_ATTESTATION: "/synthetic-attestation.json",
      DEX_LIVE_OUTPUT: "/synthetic-output.json",
      DEX_LIVE_SHARD: "0",
      DEX_LIVE_ISSUE: native ? "1505" : "1499",
    },
    stdout: { write() {} }, stderr: { write() {} },
  };
  const result = vm.runInNewContext(source, {
    __dirname,
    process: fakeProcess,
    require(name) {
      if (name === "node:fs") return fakeFs;
      if (name === "node:module") return {
        createRequire: () => () => ({ chromium: { launch() { chrome++; throw Error("No Chrome allowed"); } } }),
      };
      return require(name);
    },
    fetch() { network++; throw Error("No network allowed"); },
  }, { timeout: 1000 });
  await result;
  assert.equal(network, 0);
  assert.equal(chrome, 0);
  assert.equal(receipt.overall, "FAIL", "independent attestation remains mandatory");
  return receipt;
}

for (const native of [false, true]) {
  test(`actual driver wires ${native ? "native v9" : "generic v8"} budgets without changing clocks or acceptance scope`, async () => {
    const r = await guardedDriverReceipt(native);
    assert.equal(r.qa_measurement_version, quoteBudgetProfile(native).measurementVersion);
    assert.equal(r.shard_quote_progress_budget, native ? 800 : 600);
    assert.equal(r.cohort_quote_progress_budget, native ? 800 : 1800);
    assert.equal(r.context_quote_progress_maximum, 600);
    assert.equal(r.shard_count, native ? 1 : 3);
    assert.equal(r.expected_window_count, 4);
    assert.equal(r.duration_per_case_ms, 190000);
    assert.equal(r.browser_budget_ms, 750);
    assert.equal(r.socket_budget_ms, 500);
    assert.equal(r.concurrency_per_browser, 1);
    assert.equal(r.measurement_clock, "original_input_event_through_two_render_frames");
  });
}
