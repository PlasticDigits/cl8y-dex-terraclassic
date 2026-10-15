const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const http = require("node:http");
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
          issue === "1505" ? 600 : 1800,
        );
        assert.equal(fs.statSync(output).mode & 0o777, 0o600);
      } finally {
        fs.rmSync(directory, { recursive: true, force: true });
      }
    },
  );
