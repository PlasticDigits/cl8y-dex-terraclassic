#!/usr/bin/env node
// Exercise the actual browser measurement closure with DOM events and controlled frames.
// No browser, public request, wallet, credentials or deployment is involved.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { createRequire } = require("node:module");
const { test } = require("node:test");
const root = path.resolve(__dirname, "../..");
const requireFrontend = createRequire(path.join(root, "frontend-dapp/package.json"));
const { JSDOM } = requireFrontend("jsdom");
const { parse } = requireFrontend("@babel/parser");
const source = fs.readFileSync(path.join(__dirname, "browser-live-trading-quotes-1473.cjs"), "utf8");
const nodes = [];
function visit(node) {
  if (!node || typeof node !== "object") return;
  if (typeof node.type === "string") nodes.push(node);
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) value.forEach(visit);
    else if (value && typeof value === "object") visit(value);
  }
}
visit(parse(source));
const text = (node) => source.slice(node.start, node.end);
const browserValidation = nodes.find((node) => node.type === "FunctionDeclaration" && node.id.name === "validate");
assert.ok(browserValidation, "test must execute the actual browser quote validator");
const integerDefinition = nodes.find((node) => node.type === "VariableDeclarator" && node.id.name === "integer");
const decimalDefinition = nodes.find((node) => node.type === "VariableDeclarator" && node.id.name === "decimal");
const validateQuote = new Function(`const S = 10n ** 18n, M = (1n << 128n) - 1n;
  const ${text(integerDefinition)}, ${text(decimalDefinition)};
  return (${text(browserValidation)});`)();
// Reuse the independent finite-size economic example, not production arithmetic.
const economicsSource = fs.readFileSync(path.join(__dirname, "test-live-quote-economics-1500.cjs"), "utf8");
const economicsFixture = parse(economicsSource).program.body.find((node) => node.type === "FunctionDeclaration" && node.id.name === "fixture");
const quoteFixture = new Function(`return (${economicsSource.slice(economicsFixture.start, economicsFixture.end)})`)();
test("actual browser validator keeps 30-minute signing and 20-second producer admission separate", () => {
  const f = quoteFixture();
  f.body.execution_metadata.expires_at_ms = 1801000;
  assert.equal(validateQuote(f.body, f.params, 60000), true);
  assert.equal(validateQuote(f.body, f.params, 1801000), false);
  f.body.snapshot_age_ms = 20000;
  assert.equal(validateQuote(f.body, f.params, 60000), true);
  for (const age of [20001, -1, true, "1000", undefined]) {
    f.body.snapshot_age_ms = age;
    assert.equal(validateQuote(f.body, f.params, 60000), false);
  }
  f.body.snapshot_age_ms = 1000;
  f.body.execution_metadata.expires_at_ms = 1801001;
  assert.equal(validateQuote(f.body, f.params, 60000), false);
});
const measurement = nodes.find((node) => node.type === "ArrowFunctionExpression" && node.params[0]?.name === "expected" && text(node).includes("window.__measurement1473"));
assert.ok(measurement, "test must execute the production measurement closure");
const viewportSetup = nodes.find((node) => node.type === "FunctionDeclaration" && node.id.name === "prepareMeasurementViewport");
assert.ok(viewportSetup, "test must execute the production viewport setup");
const prepareViewport = new Function(`return (${text(viewportSetup)})`)();
const failureExpression = nodes.find((node) => node.type === "AssignmentExpression" && node.left.type === "MemberExpression" && node.left.object.name === "row" && node.left.property.name === "failed");
const budget = nodes.find((node) => node.type === "VariableDeclarator" && node.id.name === "BROWSER_LATENCY_BUDGET_MS").init.value;
assert.equal(budget, 750, "retain the authorized original-event latency limit");
const epoch = 1_700_000_000_000;
const expected = { input: "in", output: "out", amount: "100000", spread: "0.01", decimals: 6, offerDecimals: 6 };

function quote(overrides = {}) {
  return { started: 20, headers_received: 290, ended: 300, status: 200, classified: true, useful: true, output: "200000", minimum: "198000", observed: epoch - 1000, expires: epoch + 19000, params: { token_in: "in", token_out: "out", amount_in: "100000", max_spread: "0.01", protection_policy: "quote_relative_v1", trader: "private-actor-sentinel" }, generation: "private-context-digest-sentinel", url: "private-endpoint-sentinel", ...overrides };
}

function fixture({ receiveTop = 200, error = "", waiting = false, startInput = true, width = 1440, height = 1000 } = {}) {
  const dom = new JSDOM(`<input data-testid="swap-you-pay-amount"><input data-testid="swap-you-receive"><p data-testid="swap-quote-error"></p><div data-testid="swap-price-data-loading"><span>Loading price…</span><progress aria-label="Loading price"></progress></div>`, { runScripts: "outside-only" });
  const w = dom.window;
  const pay = w.document.querySelector('[data-testid="swap-you-pay-amount"]');
  const receive = w.document.querySelector('[data-testid="swap-you-receive"]');
  const alert = w.document.querySelector('[data-testid="swap-quote-error"]');
  const loading = w.document.querySelector('[data-testid="swap-price-data-loading"]');
  alert.textContent = error;
  if (!waiting) loading.remove();
  let now = 0, nextId = 1;
  const frames = new Map(), timers = new Map();
  Object.defineProperty(w.performance, "now", { value: () => now });
  w.Date.now = () => epoch + now;
  w.requestAnimationFrame = (callback) => { const id = nextId++; frames.set(id, callback); return id; };
  w.cancelAnimationFrame = (id) => frames.delete(id);
  w.setInterval = (callback) => { const id = nextId++; timers.set(id, callback); return id; };
  w.clearInterval = (id) => timers.delete(id);
  Object.defineProperty(w, "innerHeight", { value: height });
  Object.defineProperty(w, "innerWidth", { value: width });
  const rect = (top) => ({ x: 100, y: top, left: 100, top, right: 500, bottom: top + 32, width: 400, height: 32 });
  receive.getBoundingClientRect = () => rect(receiveTop);
  pay.getBoundingClientRect = () => rect(100);
  alert.getBoundingClientRect = () => rect(100);
  loading.getBoundingClientRect = () => rect(100);
  for (const element of [pay, receive, alert, loading]) element.style.opacity = "1";
  w.__quotes1473 = [];
  w.eval(`(${text(measurement)})`)(expected);
  const input = () => {
    pay.value = "0.1";
    pay.dispatchEvent(new w.Event("input", { bubbles: true }));
  };
  if (startInput) input();
  const tick = (at) => { now = at; [...timers.values()].forEach((callback) => callback()); };
  const frame = (at) => { now = at; const callbacks = [...frames.values()]; frames.clear(); callbacks.forEach((callback) => callback(at)); };
  return { w, pay, receive, alert, input, tick, frame, frames, timers, moveReceive: (top) => { receiveTop = top; }, result: () => JSON.parse(JSON.stringify(w.__measurement1473.read())), close: () => w.close() };
}

function failed(row) {
  return new Function("row", "BROWSER_LATENCY_BUDGET_MS", `return ${text(failureExpression.right)}`)(row, budget);
}

function setupPage(f, onSettingsClick) {
  return {
    locator(selector) {
      assert.equal(selector, "#swap-slippage-settings");
      return {
        async isVisible() {
          const panel = f.w.document.querySelector(selector);
          return panel != null && !panel.hidden;
        },
        async waitFor({ state }) {
          assert.equal(state, "hidden");
          const panel = f.w.document.querySelector(selector);
          assert.ok(panel == null || panel.hidden, "unmounted settings are hidden");
        },
      };
    },
    getByRole(role, options) {
      assert.equal(role, "button");
      assert.deepEqual(options, { name: "Settings", exact: true });
      return {
        async click() {
          onSettingsClick();
        },
      };
    },
    getByTestId(id) {
      assert.equal(id, "swap-you-pay-amount");
      return {
        async evaluate(callback) {
          return callback(f.pay);
        },
      };
    },
    async waitForFunction(callback) {
      assert.equal(
        f.w.eval(`(${callback.toString()})()`),
        true,
        "both real DOM inputs must intersect before the clock",
      );
    },
  };
}

test("expanded phone settings are closed before the original input clock and visible useful output", async () => {
  const f = fixture({
    receiveTop: 1060,
    startInput: false,
    width: 390,
    height: 844,
  });
  const panel = f.w.document.createElement("div");
  panel.id = "swap-slippage-settings";
  f.w.document.body.append(panel);
  let clicks = 0;
  f.w.scrollBy = ({ behavior }) => {
    assert.equal(behavior, "instant");
  };
  f.w.__quotes1473.push(quote());
  f.receive.value = "0.2";
  await prepareViewport(
    setupPage(f, () => {
      clicks++;
      panel.remove();
      f.moveReceive(640);
    }),
  );
  assert.equal(clicks, 1);
  assert.equal(
    f.w.__measurement1473.started,
    null,
    "setup must not start or reset the measured event",
  );
  f.input();
  f.tick(334);
  f.frame(336);
  f.frame(352);
  assert.equal(f.result().useful, true);
  assert.equal(f.result().input_to_dom_ms, 352);
  f.close();
});

test("closed settings still require both inputs on screen before input without post-input scrolling", async () => {
  const f = fixture({ startInput: false, width: 390, height: 844 });
  assert.equal(f.w.document.querySelector("#swap-slippage-settings"), null);
  let scroll = 0,
    calls = 0;
  const rect = (top) => ({
    left: 20,
    right: 370,
    top: top - scroll,
    bottom: top + 32 - scroll,
    width: 350,
    height: 32,
  });
  f.pay.getBoundingClientRect = () => rect(900);
  f.receive.getBoundingClientRect = () => rect(1300);
  f.w.scrollBy = ({ top, behavior }) => {
    assert.equal(f.w.__measurement1473.started, null);
    assert.equal(behavior, "instant");
    scroll += top;
    calls++;
  };
  await prepareViewport(
    setupPage(f, () => assert.fail("closed settings must not reopen")),
  );
  assert.equal(calls, 1);
  f.w.__quotes1473.push(quote());
  f.receive.value = "0.2";
  f.input();
  f.tick(300);
  f.frame(316);
  f.frame(333);
  assert.equal(f.result().useful, true);
  assert.equal(
    calls,
    1,
    "measurement itself must not scroll after the original event",
  );
  f.close();
});

test("viewport setup rejects hidden or transparent receive fields before starting input", async () => {
  for (const style of [{ opacity: "0" }, { visibility: "hidden" }]) {
    const f = fixture({ startInput: false, width: 390, height: 844 });
    Object.assign(f.receive.style, style);
    f.w.scrollBy = () => {};
    await assert.rejects(
      prepareViewport(setupPage(f, () => assert.fail("unmounted settings must not reopen"))),
      /both real DOM inputs must intersect before the clock/,
    );
    assert.equal(f.w.__measurement1473.started, null);
    assert.equal(f.w.__measurement1473.done, null);
    f.close();
  }
});

test("visible dust error confirms a blocked outcome with a blank receive below the viewport", () => {
  const f = fixture({ receiveTop: 1006, error: "This amount is too small to receive a positive output." });
  f.w.__quotes1473.push(quote({ status: 422, useful: false, reason: "amount_too_small", output: undefined }));
  f.tick(337);
  f.frame(340);
  assert.equal(f.w.__measurement1473.done, null, "first frame cannot finish measurement");
  f.frame(356);
  const result = f.result();
  assert.equal(result.classified, true);
  assert.equal(result.useful, false);
  assert.equal(result.outcome, "amount_too_small");
  assert.equal(result.input_to_dom_ms, 356);
  assert.equal(failed(result), false);
  assert.equal(f.frames.size + f.timers.size, 0, "cleanup cancels all measurement work");
  f.close();
});

test("an unrelated error cannot satisfy the typed rejection", () => {
  const f = fixture({ error: "Trading is restricted for this route." });
  f.w.__quotes1473.push(quote({ status: 422, useful: false, reason: "amount_too_small", output: undefined }));
  f.tick(337); f.frame(340); f.frame(356);
  assert.equal(f.result().classified, false);
  f.close();
});

test("missing observations may confirm an accessible waiting surface, never useful output", () => {
  const f = fixture({ receiveTop: 1006, waiting: true });
  f.receive.setAttribute("aria-busy", "true");
  f.w.__quotes1473.push(quote({ status: 503, useful: false, reason: "missing_observations", output: undefined }));
  f.tick(337); f.frame(340); f.frame(356);
  const result = f.result();
  assert.equal(result.classified, true);
  assert.equal(result.useful, false);
  assert.equal(result.outcome, "missing_observations");
  f.close();
});

test("useful output needs the exact validated amount visible through both frames", () => {
  const f = fixture({ receiveTop: 1006 });
  f.w.__quotes1473.push(quote());
  f.receive.value = "0.2";
  f.tick(300); f.frame(316); f.frame(333);
  assert.equal(f.w.__measurement1473.done, null, "offviewport useful amount must remain pending");
  f.moveReceive(200);
  f.receive.value = "0.19";
  f.tick(400); f.frame(416); f.frame(433);
  assert.equal(f.w.__measurement1473.done, null, "a different amount cannot satisfy the quote");
  f.receive.value = "0.2";
  f.tick(720); f.frame(734); f.frame(750);
  const result = f.result();
  assert.equal(result.useful, true);
  assert.equal(result.input_to_dom_ms, 750);
  assert.equal(failed(result), false, "the exact limit is inclusive");
  f.close();
});

test("an unvalidated or busy receive field cannot become useful output", () => {
  const f = fixture();
  const row = quote({ classified: false, useful: false, output: undefined });
  f.w.__quotes1473.push(row);
  f.receive.value = "0.2";
  f.tick(300); f.frame(316); f.frame(333);
  assert.equal(f.w.__measurement1473.done, null);
  Object.assign(row, quote());
  f.receive.setAttribute("aria-busy", "true");
  f.tick(400); f.frame(416); f.frame(433);
  assert.equal(f.w.__measurement1473.done, null);
  f.receive.removeAttribute("aria-busy");
  f.tick(700); f.frame(716); f.frame(733);
  assert.equal(f.result().useful, true);
  f.close();
});

test("expired cache and a late fresh response preserve the first input clock and sanitized diagnostics", () => {
  const f = fixture();
  f.receive.value = "0.2";
  f.w.__quotes1473.push(quote({ started: -7000, headers_received: -6700, ended: -6699, expires: epoch - 1106 }));
  f.tick(36); f.frame(40); f.frame(56);
  assert.equal(f.w.__measurement1473.done, null, "expired cached output cannot become useful");
  const active = quote({ started: 10000, headers_received: undefined, ended: undefined, status: null, classified: undefined, useful: undefined, output: undefined });
  f.w.__quotes1473.push(active);
  f.tick(10001); f.frame(10016); f.frame(10033);
  Object.assign(active, quote({ started: 10000, headers_received: 10300, ended: 10301 }));
  f.tick(10302); f.frame(10317); f.frame(10334);
  const result = f.result();
  assert.equal(result.input_to_dom_ms, 10334);
  assert.equal(failed(result), true);
  assert.equal(result.diagnostics.previous_records[0].expiry_at_ms, -1106);
  assert.ok(result.diagnostics.timeline.some((state) => state.expired_prior_quote && state.quote_request_count === 0));
  assert.ok(result.diagnostics.timeline.some((state) => state.quote_request_count === 1 && !state.response_received));
  assert.equal(result.diagnostics.current_records[0].request_at_ms, 10000);
  assert.equal(result.diagnostics.current_records[0].headers_at_ms, 10300);
  assert.doesNotMatch(JSON.stringify(result), /private-(actor|context|endpoint)/);
  f.close();
});

test("failed-row diagnostics bound request records and state transitions", () => {
  const f = fixture();
  for (let index = 0; index < 40; index++) f.w.__quotes1473.push(quote({ started: index, headers_received: undefined, ended: undefined, status: null, useful: false }));
  for (let index = 0; index < 40; index++) {
    f.receive.setAttribute("aria-busy", String(index % 2 === 0));
    f.tick(100 + index);
  }
  const result = f.result();
  assert.equal(result.diagnostics.current_records_truncated, true);
  assert.equal(result.diagnostics.current_request_count, 40);
  assert.equal(result.diagnostics.current_records.length, 12);
  assert.equal(result.diagnostics.timeline.length, 24);
  assert.ok(Buffer.byteLength(JSON.stringify(result.diagnostics)) <= 16 * 1024);
  f.close();
});

test("the actual completed-helper tail exits nonzero for executed failures", () => {
  const named = (name) => text(nodes.find((node) => node.type === "FunctionDeclaration" && node.id.name === name));
  const main = nodes.find((node) => node.type === "FunctionDeclaration" && node.id.name === "main");
  const completed = main.body.body.findIndex((node) => node.type === "ExpressionStatement" && node.expression.type === "AssignmentExpression" && node.expression.left.object?.name === "receipt" && node.expression.left.property?.name === "completed");
  assert.ok(completed >= 0);
  const tail = source.slice(main.body.body[completed].start, main.body.end - 1);
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "quote1473-helper-test-"));
  try {
    for (const sampleFailed of [false, true]) {
      const program = `const fs=require('node:fs');const OUTPUT=${JSON.stringify(path.join(directory, "receipt.json"))};const receipt={samples:[{useful:true,input_to_dom_ms:${sampleFailed ? 751 : 750},failed:${sampleFailed},viewport:'desktop'}]};const quoteRequests=0,networkQuoteRequests=0,preventedWrites=0,blockedWebSockets=0,elapsedBudgetExpired=false,QUOTE_LIMIT=2200;${named("stats")};${named("save")};${tail}`;
      const child = spawnSync(process.execPath, ["-"], { input: program, encoding: "utf8", env: {} });
      assert.equal(child.status, sampleFailed ? 1 : 0, child.stderr);
      assert.equal(JSON.parse(child.stdout).overall, sampleFailed ? "FAIL" : "PASS WITH SKIPS");
      assert.equal(JSON.parse(fs.readFileSync(path.join(directory, "receipt.json"), "utf8")).completed, true);
    }
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
