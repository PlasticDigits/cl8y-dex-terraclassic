const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const http = require("node:http");
const vm = require("node:vm");
const { createRequire } = require("node:module");
const {
  nativeResponseEpoch,
  recordWindowQuoteOutcome,
  drainResponseProbes,
  isOwnedTeardownCancellation,
} = require("./live-input-clock-1499.cjs");
const { chromium } = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
)("@playwright/test");

// Run the actual driver through its initialized phase without HTTP or Chrome.
// The launch boundary snapshots the receipt before any window can begin.
async function initializedCheckpointScenario({
  native = true,
  unavailable = false,
  serverDates = null,
} = {}) {
  const source = fs.readFileSync(
    path.join(__dirname, "live-trading-lifecycle-1499.cjs"),
    "utf8",
  );
  const entry = source.lastIndexOf("\nmain()\n");
  assert.ok(entry > 0);
  const revision = "a".repeat(40),
    attestationPath = "/fixture-attestation.json";
  const attestation = {
    overall: "PASS",
    expected_revision: revision,
    frontend: {
      source_sha: revision,
      deployment_status: "finished",
      artifact: {},
    },
    backend: { git_sha: revision },
    backend_deployment: { source_sha: revision, deployment_status: "finished" },
  };
  let wall = 1792115350000,
    monotonic = 0,
    launches = 0,
    output;
  const writes = [],
    events = [],
    calls = [];
  const dates = serverDates ?? [
    "Fri, 16 Oct 2026 01:49:10 GMT",
    null,
    "private-invalid-header",
    "x".repeat(65),
  ];
  const fileSystem = {
    ...fs,
    statSync(name) {
      return name === attestationPath ? { size: 1024 } : fs.statSync(name);
    },
    readFileSync(name, ...args) {
      return name === attestationPath
        ? JSON.stringify(attestation)
        : fs.readFileSync(name, ...args);
    },
    writeFileSync(_name, body, options) {
      writes.push({ receipt: JSON.parse(body), mode: options.mode });
    },
    chmodSync(_name, mode) {
      assert.equal(mode, 0o600);
    },
  };
  const fixture = {
    async fetch(url, options) {
      calls.push({ url, options, request_epoch_ms: wall });
      let status = 200,
        body,
        date = null;
      if (url.endsWith("/health")) body = { git_sha: revision };
      else if (url.endsWith("/api/v1/tokens"))
        body = [
          { symbol: "cLUNC", contract_address: "terra1" + "a".repeat(38) },
        ];
      else {
        const index =
          calls.filter((call) => call.url.includes("/route/solve?")).length - 1;
        status = unavailable && index >= 2 ? 503 : 200;
        body =
          status === 503
            ? { error: "missing_observations" }
            : { fixture: true };
        date = dates[index];
      }
      wall += 25;
      monotonic += 25;
      return {
        status,
        headers: {
          get(name) {
            return name === "date"
              ? date
              : name === "x-quote-request-id"
                ? "b".repeat(16)
                : null;
          },
        },
        body: (async function* () {
          wall += 40;
          monotonic += 40;
          yield Buffer.from(JSON.stringify(body));
          wall += 10;
          monotonic += 10;
        })(),
      };
    },
    chromium: {
      async launch() {
        launches++;
        // A browser launch itself cannot be mistaken for initialized success.
        assert.equal(writes.length, 1);
        assert.equal(writes[0].receipt.windows.length, 0);
        throw Error("offline_stop_at_browser_launch");
      },
    },
  };
  output = await vm.runInNewContext(
    source.slice(0, entry) +
      `
    artifacts = async () => ({});
    main().then(() => { throw Error("unexpected_browser_completion"); }, error => {
      if (error.message !== "offline_stop_at_browser_launch") throw error;
      return { receipt, quotes, readCount, routeReads, duration: DURATION, wallBudget: GLOBAL_BUDGET };
    });
  `,
    {
      __dirname,
      Buffer,
      URL,
      URLSearchParams,
      AbortSignal,
      fetch: fixture.fetch,
      performance: { now: () => monotonic },
      Date: class extends Date {
        constructor(...args) {
          super(...(args.length ? args : [wall]));
        }
        static now() {
          return wall;
        }
      },
      process: {
        env: {
          EXPECTED_SOURCE_SHA: revision,
          DEX_LIVE_ATTESTATION: attestationPath,
          DEX_LIVE_OUTPUT: "/fixture-receipt.json",
          DEX_LIVE_ISSUE: native ? "1505" : "1499",
          DEX_LIVE_SHARD: "0",
        },
        stderr: {
          write(line) {
            events.push(JSON.parse(line));
          },
        },
      },
      require(name) {
        if (name === "node:fs") return fileSystem;
        if (name === "node:module")
          return {
            createRequire: () => () => ({ chromium: fixture.chromium }),
          };
        if (name === "./live-quote-economics-1500.cjs")
          return {
            ...require(name),
            validate: () => ({ useful: true, metrics: true }),
          };
        return require(name);
      },
    },
    { timeout: 1000 },
  );
  return { output, writes, events, calls, launches };
}

test("initialized checkpoint retains two fast503 failures before browser launch without extra requests", async () => {
  const { output, writes, events, calls, launches } =
    await initializedCheckpointScenario({ unavailable: true });
  const checkpoint = writes[0].receipt.initialized_api_checkpoint;
  assert.equal(checkpoint.overall, "FAIL");
  assert.equal(checkpoint.completed, true);
  assert.equal(checkpoint.samples, 4);
  assert.equal(checkpoint.useful, 2);
  assert.equal(checkpoint.not_useful, 2);
  assert.deepEqual(Array.from(checkpoint.failures), [
    "native_socket_not_useful",
    "native_socket_not_useful",
  ]);
  assert.deepEqual(
    writes[0].receipt.api.map((row) => row.status),
    [200, 200, 503, 503],
  );
  assert.equal(writes[0].receipt.completed, false);
  assert.equal(writes[0].mode, 0o600);
  assert.equal(events.length, 1);
  assert.equal(events[0].event, "initialized_api_checkpoint");
  assert.equal(events[0].overall, "FAIL");
  assert.equal(events[0].api.length, 4);
  assert.equal(
    launches,
    1,
    "metadata checkpoint must not add fail-fast/skip semantics",
  );
  assert.equal(
    calls.length,
    6,
    "only existing2 metadata calls and4 initialized solves reach mock transport",
  );
  assert.equal(output.quotes, 4);
  assert.equal(output.readCount, 6);
  assert.equal(output.routeReads.solve, 4);
  assert.equal(output.routeReads.progress, 0);
});

test("initialized epochs use actual existing call/body completion and sanitize server Date to numeric or null", async () => {
  const { writes, calls, events } = await initializedCheckpointScenario();
  const receipt = writes[0].receipt;
  const solves = calls.filter((call) => call.url.includes("/route/solve?"));
  for (let i = 0; i < 4; i++) {
    const row = receipt.api[i];
    assert.equal(row.request_epoch_ms, solves[i].request_epoch_ms);
    assert.equal(row.response_epoch_ms, row.request_epoch_ms + 75);
    assert.equal(
      row.elapsed_ms,
      75,
      "existing monotonic useful-response clock is unchanged",
    );
    assert.ok(Number.isSafeInteger(row.request_epoch_ms));
    assert.ok(Number.isSafeInteger(row.response_epoch_ms));
    assert.equal(
      row.server_date_epoch_ms,
      i === 0 ? Date.parse("Fri, 16 Oct 2026 01:49:10 GMT") : null,
    );
    assert.equal(solves[i].options.redirect, "error");
    assert.equal(solves[i].options.headers["Cache-Control"], "no-cache");
  }
  assert.equal(receipt.qa_metadata_version, "initialized_api_checkpoint_v1");
  assert.equal(receipt.initialized_api_checkpoint.overall, "PASS");
  assert.equal(
    receipt.overall,
    undefined,
    "initialized PASS cannot promote the incomplete full cohort to PASS",
  );
  assert.ok(
    !JSON.stringify({ receipt, events }).includes("private-invalid-header"),
  );
  assert.ok(!JSON.stringify({ receipt, events }).includes("x".repeat(65)));
});

for (const [label, date] of [
  ["ISO-only", "2026-10-16"],
  ["numeric", "2026"],
  ["invalid calendar", "Tue, 31 Feb 2026 01:49:10 GMT"],
  ["wrong weekday", "Thu, 16 Oct 2026 01:49:10 GMT"],
])
  test(`initialized epochs reject parseable malformed HTTP Date (${label})`, async () => {
    const { writes, calls } = await initializedCheckpointScenario({
      serverDates: [date, null, null, null],
    });
    const receipt = writes[0].receipt;
    assert.equal(receipt.api[0].server_date_epoch_ms, null);
    assert.equal(receipt.initialized_api_checkpoint.overall, "PASS");
    assert.ok(receipt.api.every((row) => row.useful && row.status === 200));
    assert.ok(receipt.api.every((row) => row.elapsed_ms === 75));
    assert.equal(
      calls.filter((call) => call.url.includes("/route/solve?")).length,
      4,
    );
    assert.equal(calls.length, 6);
  });

test("initialized checkpoint preserves native and generic versions, budgets, defaults and typed-outcome policy", async () => {
  for (const native of [true, false]) {
    const { output, writes, calls } = await initializedCheckpointScenario({
      native,
      unavailable: true,
    });
    const receipt = writes[0].receipt;
    assert.equal(receipt.shard_quote_progress_budget, native ? 800 : 600);
    assert.equal(receipt.cohort_quote_progress_budget, native ? 800 : 1800);
    assert.equal(receipt.context_quote_progress_maximum, 600);
    assert.equal(
      receipt.qa_measurement_version,
      native
        ? "foreground_correlated_native_availability_v9"
        : "foreground_correlated_native_availability_v8",
    );
    assert.equal(receipt.expected_window_count, 4);
    assert.equal(receipt.browser_budget_ms, 750);
    assert.equal(receipt.socket_budget_ms, 500);
    assert.equal(output.duration, 190000);
    assert.equal(output.wallBudget, 16 * 60000);
    assert.equal(
      receipt.initialized_api_checkpoint.overall,
      native ? "FAIL" : "PASS",
    );
    assert.deepEqual(
      receipt.api.map((row) => row.phase),
      [
        "initialized_first",
        "initialized_retest",
        "initialized_first",
        "initialized_retest",
      ],
    );
    const amounts = calls
      .filter((call) => call.url.includes("/route/solve?"))
      .map((call) => new URL(call.url).searchParams.get("amount_in"));
    assert.equal(amounts.length, 4);
    assert.equal(amounts[0], amounts[1]);
    assert.equal(amounts[2], amounts[3]);
  }
});

test("a fast typed503 fails native sustained availability even when a later useful quote recovers", () => {
  const row = { responses: [], failures: [] };
  recordWindowQuoteOutcome(row, { status: 200, useful: true }, true);
  recordWindowQuoteOutcome(
    row,
    { status: 503, reason: "missing_observations", at_ms: 1, useful: false },
    true,
  );
  recordWindowQuoteOutcome(row, { status: 200, useful: true }, true);
  assert.deepEqual(row.failures, ["native_window_not_useful"]);
  assert.deepEqual(row.quote_availability, {
    responses: 3,
    useful: 2,
    not_useful: 1,
    teardown_cancelled: 0,
  });
  assert.equal(row.responses[1].reason, "missing_observations");
});
test("useful200s pass native windows; invalid200/body failures count as unavailable while generic retention remains allowed", () => {
  const healthy = { responses: [], failures: [] };
  recordWindowQuoteOutcome(healthy, { status: 200, useful: true }, true);
  recordWindowQuoteOutcome(healthy, { status: 200, useful: true }, true);
  assert.deepEqual(healthy.failures, []);
  assert.deepEqual(healthy.quote_availability, {
    responses: 2,
    useful: 2,
    not_useful: 0,
    teardown_cancelled: 0,
  });
  const invalid = { responses: [], failures: [] };
  recordWindowQuoteOutcome(
    invalid,
    { status: 200, failure: "original_window" },
    true,
  );
  recordWindowQuoteOutcome(
    invalid,
    { status: 503, useful: false, failure: "AbortError" },
    true,
  );
  assert.deepEqual(invalid.failures, ["native_window_not_useful"]);
  assert.equal(invalid.quote_availability.not_useful, 2);
  const generic = { responses: [], failures: [] };
  recordWindowQuoteOutcome(
    generic,
    { status: 503, reason: "missing_observations" },
    false,
  );
  assert.deepEqual(generic.failures, []);
  assert.deepEqual(generic.quote_availability, {
    responses: 1,
    useful: 0,
    not_useful: 1,
    teardown_cancelled: 0,
  });
});

test("only owned200 teardown cancellation is qualified; active cancellation and a503 overlapping close remain native failures", () => {
  for (const [status, ownedTeardown, shouldFail] of [
    [200, true, false],
    [200, false, true],
    [503, true, true],
  ]) {
    const row = {
      responses: [],
      failures: [],
      completed: true,
      elapsed_ms: 190000,
    };
    recordWindowQuoteOutcome(
      row,
      { status, useful: false, failure: "request_cancelled" },
      true,
      ownedTeardown,
    );
    assert.equal(row.failures.includes("native_window_not_useful"), shouldFail);
    assert.equal(row.quote_availability.responses, 1);
    assert.equal(row.quote_availability.teardown_cancelled, shouldFail ? 0 : 1);
    assert.equal(row.quote_availability.not_useful, shouldFail ? 1 : 0);
    assert.equal(row.completed, true);
  }
});

test("teardown never qualifies parsed invalidJSON, a real body error or an active cancellation", () => {
  const closed = new Error(
    "response.body: Target page, context or browser has been closed",
  );
  assert.equal(
    isOwnedTeardownCancellation(200, true, true, null, closed),
    true,
  );
  assert.equal(
    isOwnedTeardownCancellation(200, true, true, "net::ERR_ABORTED", closed),
    true,
  );
  assert.equal(
    isOwnedTeardownCancellation(200, true, false, "net::ERR_ABORTED", closed),
    false,
  );
  assert.equal(
    isOwnedTeardownCancellation(503, true, true, "net::ERR_ABORTED", closed),
    false,
  );
  assert.equal(
    isOwnedTeardownCancellation(
      200,
      false,
      true,
      "net::ERR_ABORTED",
      new SyntaxError("invalid JSON"),
    ),
    false,
  );
  assert.equal(
    isOwnedTeardownCancellation(
      200,
      true,
      true,
      null,
      new Error("body read failed"),
    ),
    false,
  );
});

test("actual Chrome in-flight200 is drained within a finite bound then qualified on deliberate context teardown", async () => {
  const server = http.createServer((req, res) => {
    if (req.url === "/quote") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.write('{"ok":');
    } else {
      res.setHeader("Content-Type", "text/html");
      res.end("<!doctype html><title>Owned teardown fixture</title>");
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  let browser;
  try {
    browser = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
    });
    const context = await browser.newContext(),
      page = await context.newPage();
    const origin = "http://127.0.0.1:" + server.address().port;
    await page.goto(origin);
    const ready = page.waitForResponse(origin + "/quote");
    await page.evaluate(() => {
      fetch("/quote")
        .then((r) => r.text())
        .catch(() => {});
    });
    const response = await ready,
      row = { responses: [], failures: [] };
    let ownedTeardown = false;
    const outcome = response.body().then(
      () => recordWindowQuoteOutcome(row, { status: 200, useful: true }, true),
      (error) =>
        recordWindowQuoteOutcome(
          row,
          { status: response.status(), failure: "request_cancelled" },
          true,
          isOwnedTeardownCancellation(
            response.status(),
            true,
            ownedTeardown,
            response.request().failure()?.errorText,
            error,
          ),
        ),
    );
    const pending = new Set([outcome]);
    assert.equal(await drainResponseProbes(pending, 10), false);
    ownedTeardown = true;
    await context.close();
    assert.equal(await drainResponseProbes(pending, 1000), true);
    assert.deepEqual(row.failures, []);
    assert.equal(row.quote_availability.teardown_cancelled, 1);
    assert.equal(row.responses[0].phase, "teardown");
  } finally {
    if (browser) await browser.close();
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
});

test("actual Chrome response timing stays at native completion when Node diagnostic body reading is delayed", async () => {
  const server = http.createServer((req, res) => {
    res.setHeader(
      "Content-Type",
      req.url === "/" ? "text/html" : "application/json",
    );
    res.end(
      req.url === "/"
        ? "<!doctype html><title>Owned clock fixture</title>"
        : '{"ok":true}',
    );
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  let browser;
  try {
    browser = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
    });
    const page = await browser.newPage();
    const origin = "http://127.0.0.1:" + server.address().port;
    await page.goto(origin);
    const responseReady = page.waitForResponse(origin + "/quote");
    await page.evaluate(async () => {
      await (await fetch("/quote")).text();
    });
    const response = await responseReady;
    await response.finished();
    const originalCompletion = nativeResponseEpoch(response.request().timing());
    const diagnosticStart = performance.now();
    await new Promise((resolve) => setTimeout(resolve, 250));
    assert.equal((await response.body()).toString(), '{"ok":true}');
    // The consumer microtask and Chrome's network-complete event have separate
    // delivery order. Test stable native timing across delayed CDP retrieval,
    // rather than assuming a two-ms ordering between those browser events.
    assert.equal(
      nativeResponseEpoch(response.request().timing()),
      originalCompletion,
    );
    assert.ok(performance.now() - diagnosticStart >= 240);
  } finally {
    if (browser) await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
});

for (const issue of ["1499", "1505"])
  test(
    "missing exact attestation refuses before any live read and writes a private bounded failure receipt (#" +
      issue +
      ")",
    () => {
      const directory = fs.mkdtempSync(
        path.join(os.tmpdir(), "quote-1499-guard-"),
      );
      const output = path.join(directory, "receipt.json");
      try {
        const run = spawnSync(
          process.execPath,
          [path.join(__dirname, "live-trading-lifecycle-1499.cjs")],
          {
            env: {
              ...process.env,
              EXPECTED_SOURCE_SHA: "",
              DEX_LIVE_ATTESTATION: "",
              DEX_LIVE_OUTPUT: output,
              DEX_LIVE_ISSUE: issue,
              DEX_LIVE_SHARD: "0",
            },
            encoding: "utf8",
            timeout: 10000,
          },
        );
        assert.equal(run.status, 1);
        assert.equal(run.stderr, "");
        const receipt = JSON.parse(fs.readFileSync(output, "utf8"));
        assert.equal(receipt.overall, "FAIL");
        assert.equal(receipt.completed, false);
        assert.equal(receipt.read_count, 0);
        assert.equal(receipt.quote_reads, 0);
        assert.equal(receipt.windows.length, 0);
        assert.deepEqual(
          receipt.issues,
          issue === "1505" ? [1505] : [1498, 1499, 1500],
        );
        assert.equal(receipt.cohort_maximum_windows, issue === "1505" ? 4 : 12);
        assert.equal(
          receipt.cohort_quote_progress_budget,
          issue === "1505" ? 800 : 1800,
        );
        assert.equal(receipt.context_quote_progress_maximum, 600);
        assert.equal(
          receipt.qa_measurement_version,
          issue === "1505"
            ? "foreground_correlated_native_availability_v9"
            : "foreground_correlated_native_availability_v8",
        );
        assert.equal(fs.statSync(output).mode & 0o777, 0o600);
      } finally {
        fs.rmSync(directory, { recursive: true, force: true });
      }
    },
  );
