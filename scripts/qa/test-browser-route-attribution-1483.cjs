#!/usr/bin/env node
// Actual shared validator/measurement and product explanation under a controlled DOM clock.
const assert = require("node:assert/strict");
const { test } = require("node:test");
const path = require("node:path");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const { spawnSync } = require("node:child_process");
const { createRequire } = require("node:module");
const frontend = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
);
const { JSDOM } = frontend("jsdom");
const {
  sharedSources,
  checkAttestation,
  checkRootPaths,
  rootScriptPaths,
  checkServedArtifact,
  toolSourceProvenance,
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
        root_scripts: [
          "/bootstrap/theme.js",
          "/bootstrap/trade-shell.js",
          "/assets/index-Bt81tS2x.js",
        ].map((path) => ({
          asset: path.split("/").at(-1),
          path,
          sha256: "c".repeat(64),
          bytes: 100,
        })),
      },
    },
    backend: { git_sha: sha },
    backend_deployment: { source_sha: sha, deployment_status: "finished" },
  };
}
function quote() {
  const now = epoch;
  const q = {
    snapshot_age_ms: 1000,
    token_in: "in",
    token_out: "out",
    estimated_amount_out_net: "890000",
    estimated_amount_out: "890000",
    spot_amount_out: "1000000",
    execution_metadata: {
      policy: "pool_only",
      protection_policy: "quote_relative_v1",
      max_spread: "0.05",
      input_amount: "1000000",
      minimum_receive: "845500",
      observed_at_ms: now - 1000,
      expires_at_ms: now + 19000,
      generation: "original",
      execution_market_spread: "0.052631578947368422",
      hops: [
        {
          offer_amount: "1000000",
          reserve_in: "1000000000000000000000000",
          reserve_out: "890000999999999999999999",
          book_side: "bid",
          book_terms: [],
          book_commission_amount: "0",
          pool_return_amount: "890000",
          commission_amount: "0",
          book_return_net: "0",
          pool_input_actual: "1000000",
          book_offer_consumed: "0",
          spread_amount: "0",
          return_amount: "890000",
          min_return: "845500",
        },
      ],
    },
    router_operations: [{ terra_swap: { min_return: "845500" } }],
    economic_attribution: {
      version: 1,
      status: "available",
      method: "spot_bridge_v1",
      generation: "original",
      observed_at_ms: now - 1000,
      expires_at_ms: now + 19000,
      fair_amount_out: "1000000",
      net_amount_out: "890000",
      components: {
        wrapper_loss: "0",
        trading_cost: "0",
        market_movement: "0",
        reference_divergence: "110000",
      },
      dominant_cause: "reference_divergence",
    },
  };
  const m = q.execution_metadata,
    h = m.hops[0];
  q.order_economics = {
    version: 1,
    status: "available",
    method: "selected_route_fee_free_v1",
    generation: m.generation,
    observed_at_ms: m.observed_at_ms,
    expires_at_ms: m.expires_at_ms,
    input_amount: m.input_amount,
    policy: m.policy,
    max_spread: m.max_spread,
    router_operations: q.router_operations,
    baseline_amount_out: q.estimated_amount_out,
    fee_free_amount_out: q.estimated_amount_out,
    order_movement: "0",
    trading_fee: "0",
    tax_hook_cost: "0",
    wrapper_cost: "0",
    reference_divergence: "110000",
    stages: [
      {
        reserve_in: h.reserve_in,
        reserve_out: h.reserve_out,
        input_amount: m.input_amount,
        output_amount: q.estimated_amount_out,
        book_input: "0",
        max_maker_fills: 1,
        book_side: "bid",
        book: [],
      },
    ],
  };
  return q;
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
    `<input data-testid="${payId}">${entry === "trade" ? `<span data-testid="${receiveId}"></span>` : `<input data-testid="${receiveId}">`}<p role="status" data-testid="${prefix}-impact-aside"></p>`,
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
    `window.__validate1483 = ${shared.validator}; window.__explain1483 = ${shared.explanation}; window.__precise1483 = ${shared.precision}; window.__format1483 = ${shared.format}; window.__riskPolicy1483 = ${shared.risk}; (${installRuntime.toString()})(${JSON.stringify({ copies: COPIES })});`,
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
    output: "890000",
    minimum: "845500",
    observed: epoch - 1000,
    expires: epoch + 19000,
    diagnostic: {
      valid: true,
      percent: "11.00",
      precise_percent: 11,
      explanation: w.__explain1483(q),
      risk: w.__riskPolicy1483.evaluateTradingRisk({
        return_amount: q.estimated_amount_out,
        executionMetadata: q.execution_metadata,
        riskEvidence: q,
      }),
    },
  };
  w.__quotes1473.push(row);
  if (entry === "trade") receive.textContent = "0.89 UST1";
  else receive.value = "0.89";
  aside.textContent = "Fair-reference deviation 11.00%.";
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
      a.frontend.artifact.root_scripts[0].path = "/assets/../private.js";
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
test("all bootstrap and canonical asset roots are parsed, ordered and hashed", () => {
  const roots = attestation().frontend.artifact.root_scripts;
  const paths = roots.map((row) => row.path);
  assert.deepEqual(
    rootScriptPaths(
      `<SCRIPT SRC=${paths[0]}></SCRIPT><script src='${paths[1]}'></script><script type="module" src="${paths[2]}"></script>`,
    ),
    paths,
  );
  assert.deepEqual(checkRootPaths([...paths, "/assets/extra-123.js"]), [
    ...paths,
    "/assets/extra-123.js",
  ]);
  const artifact = attestation().frontend.artifact;
  assert.doesNotThrow(() =>
    checkServedArtifact(structuredClone(artifact), artifact),
  );
  for (const mutate of [
    (a) => {
      a.html_sha256 = "d".repeat(64);
    },
    (a) => {
      a.root_scripts[0].sha256 = "d".repeat(64);
    },
    (a) => {
      a.root_scripts[1].sha256 = "d".repeat(64);
    },
    (a) => {
      a.root_scripts[2].sha256 = "d".repeat(64);
    },
    (a) => {
      a.root_scripts[0].path = "/assets/theme.js";
    },
    (a) => {
      a.root_scripts.reverse();
    },
    (a) => {
      a.root_scripts[2].bytes++;
    },
  ]) {
    const actual = structuredClone(artifact);
    mutate(actual);
    assert.throws(
      () => checkServedArtifact(actual, artifact),
      /ServedArtifact/,
    );
  }
});

test("root manifests reject unknown, foreign, traversal, duplicate and missing scripts", () => {
  const paths = attestation().frontend.artifact.root_scripts.map(
    (row) => row.path,
  );
  for (const bad of [
    "/bootstrap/unknown.js",
    "/bootstrap/Theme.js",
    "/assets/../private.js",
    "/assets/%2e%2e.js",
    "/assets/.hidden.js",
    "/assets/index.js?x=1",
    "/assets/index.js#x",
    "assets/index.js",
    "https://dex.cl8y.com/assets/index.js",
    "//evil.test/assets/index.js",
    "/assets/" + "x".repeat(160) + ".js",
    "",
  ]) {
    assert.throws(() => checkRootPaths([...paths, bad]), /RootScripts/);
    assert.throws(
      () =>
        rootScriptPaths(
          paths.map((p) => `<script src="${p}"></script>`).join("") +
            `<SCRIPT SRC="${bad}"></SCRIPT>`,
        ),
      /RootScripts/,
    );
  }
  for (const bad of [
    [],
    paths.slice(1),
    paths.slice(0, 2),
    [...paths, paths[0]],
    [...paths, "/assets/theme.js"],
    [...paths, ...Array.from({ length: 6 }, (_, i) => `/assets/extra${i}.js`)],
  ]) {
    assert.throws(() => checkRootPaths(bad), /RootScripts/);
  }
  assert.throws(
    () =>
      rootScriptPaths(
        '<base href="https://evil.test/">' +
          paths.map((p) => `<script src=${p}></script>`).join(""),
      ),
    /RootBase/,
  );
  assert.throws(
    () => rootScriptPaths("x".repeat(128 * 1024 + 1)),
    /RootHtmlBudget/,
  );
});

test("attested root paths, basenames, byte bounds and duplicate identity are mandatory", () => {
  for (const mutate of [
    (a) => {
      delete a.root_scripts[0].path;
    },
    (a) => {
      a.root_scripts[0].asset = "different.js";
    },
    (a) => {
      a.root_scripts[0].bytes = 0;
    },
    (a) => {
      a.root_scripts[0].bytes = 8 * 1024 * 1024 + 1;
    },
    (a) => {
      a.root_scripts[0].bytes = 1.5;
    },
    (a) => {
      a.root_scripts.push(a.root_scripts[0]);
    },
  ]) {
    const a = attestation();
    mutate(a.frontend.artifact);
    assert.throws(() => checkAttestation(a, sha));
  }
});

test("tool provenance hashes exact executed bytes and reports dirty source independently", () => {
  const source = Buffer.from("tool source");
  const clean = toolSourceProvenance(source, Buffer.from(source), sha);
  assert.equal(
    clean.tool_sha256,
    crypto.createHash("sha256").update(source).digest("hex"),
  );
  assert.equal(clean.tool_dirty, false);
  assert.equal(clean.tool_git_head, sha);
  assert.equal(
    toolSourceProvenance(source, Buffer.from("older tool"), sha).tool_dirty,
    true,
  );
  assert.equal(toolSourceProvenance(source, null, null).tool_dirty, true);
});

test("actual prerequisite-failure receipt retains tool hash and dirty provenance", () => {
  const directory = fs.mkdtempSync(
    path.join(os.tmpdir(), "dex1483-root-receipt-"),
  );
  const attestationFile = path.join(directory, "attestation.json");
  const output = path.join(directory, "receipt.json");
  const tool = path.join(__dirname, "browser-route-attribution-1483.cjs");
  try {
    fs.writeFileSync(attestationFile, "{}", { mode: 0o600 });
    const result = spawnSync(process.execPath, [tool], {
      env: {
        PATH: process.env.PATH,
        EXPECTED_SOURCE_SHA: sha,
        DEX1483_ATTESTATION_FILE: attestationFile,
        DEX1483_BROWSER_OUTPUT: output,
      },
      timeout: 10000,
      maxBuffer: 4096,
    });
    assert.equal(result.status, 1);
    const receipt = JSON.parse(fs.readFileSync(output, "utf8"));
    assert.equal(receipt.failure_class, "PrerequisiteValidation");
    assert.equal(
      receipt.tool_sha256,
      crypto.createHash("sha256").update(fs.readFileSync(tool)).digest("hex"),
    );
    assert.equal(typeof receipt.tool_dirty, "boolean");
    assert.match(receipt.tool_git_head, /^[a-f0-9]{40}$/);
    assert.equal(receipt.read_only, true);
    assert.equal(receipt.signing, false);
    assert.deepEqual(receipt.samples, []);
    assert.equal(fs.statSync(output).mode & 0o777, 0o600);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
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
      "Fair-reference deviation 11.00%. 2%",
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

test("ordinary price detail stays behind DETAILS without an outside warning", () => {
  const f = fixture();
  f.aside.remove();
  const detail = f.w.document.createElement("details");
  detail.dataset.testid = "swap-details";
  const metric = f.w.document.createElement("p");
  metric.dataset.testid = "swap-price-details";
  metric.textContent = "Fair-reference deviation 1.00%.";
  detail.appendChild(metric);
  f.w.document.body.appendChild(detail);
  f.row.diagnostic.percent = "1.00";
  f.row.diagnostic.precise_percent = 1;
  const q = f.q,
    a = q.economic_attribution,
    d = q.order_economics;
  q.spot_amount_out = a.fair_amount_out = "898989";
  a.components.reference_divergence = d.reference_divergence = "8989";
  f.row.diagnostic.risk = f.w.__riskPolicy1483.evaluateTradingRisk({
    return_amount: q.estimated_amount_out,
    executionMetadata: q.execution_metadata,
    riskEvidence: q,
  });
  f.tick(300);
  f.frame(320);
  f.frame(340);
  assert.equal(f.result().useful, true);
  assert.equal(detail.hasAttribute("open"), false);
  assert.equal(f.w.document.querySelector('[role="alert"]'), null);
  f.close();
});
test("display metric formatter retains the real safety transitions", () => {
  const f = fixture();
  assert.equal(f.w.__format1483(5.000000000000001), ">5");
  assert.equal(f.w.__format1483(30.000000000000004), ">30");
  assert.equal(f.w.__format1483(98.99999999999997), "<99");
  f.close();
});

test("actual bundled risk policy uses exact raw 5/10/30/99 and rejects forged diagnostics", () => {
  const f = fixture(),
    policy = f.w.__riskPolicy1483;
  const p = policy.exactTradingRiskPredicates({
    movement: "334",
    baseline: "10000",
    net: "9309",
    fair: "10000",
  });
  assert.equal(p.orderWarning || p.totalWarning, false);
  for (const offset of [-1n, 0n, 1n]) {
    const b = 10n ** 36n;
    const exact = policy.exactTradingRiskPredicates({
      movement: String(b / 20n + offset),
      baseline: String(b),
      net: String(b - b / 10n - offset),
      fair: String(b),
    });
    assert.equal(exact.orderWarning, offset > 0n);
    assert.equal(exact.totalWarning, offset > 0n);
  }
  for (const favorable of [false, true])
    for (const offset of [-1n, 0n, 1n]) {
      const b = 10n ** 36n;
      const exact = (difference) =>
        policy.exactTradingRiskPredicates({
          movement: "-1",
          baseline: String(b),
          net: String(favorable ? b : b - difference),
          fair: String(favorable ? b - difference : b),
        });
      assert.equal(
        exact((b * 30n) / 100n + offset).expertRequired,
        offset > 0n,
      );
      assert.equal(exact((b * 99n) / 100n + offset).extreme, offset >= 0n);
      assert.equal(exact((b * 99n) / 100n + offset).orderWarning, false);
    }
  const q = f.q;
  q.order_economics.order_movement = "0.1";
  assert.equal(
    policy.evaluateTradingRisk({
      return_amount: q.estimated_amount_out,
      executionMetadata: q.execution_metadata,
      riskEvidence: q,
    }).availability,
    "unavailable",
  );
  assert.ok(
    shared.dependencies.includes("frontend-dapp/src/utils/tradingRisk.ts"),
  );
  assert.ok(
    shared.dependencies.includes("frontend-dapp/src/utils/localQuoteWindow.ts"),
  );
  f.close();
});
