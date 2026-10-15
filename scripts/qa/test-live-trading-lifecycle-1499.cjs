const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const http = require("node:http");
const { createRequire } = require("node:module");
const { nativeResponseEpoch } = require("./live-input-clock-1499.cjs");
const { chromium } = createRequire(
  path.resolve(__dirname, "../../frontend-dapp/package.json"),
)("@playwright/test");

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
    const consumedAt = await page.evaluate(async () => {
      await (await fetch("/quote")).text();
      return Date.now();
    });
    const response = await responseReady;
    await response.finished();
    const originalCompletion = nativeResponseEpoch(response.request().timing());
    await new Promise((resolve) => setTimeout(resolve, 250));
    assert.equal((await response.body()).toString(), '{"ok":true}');
    assert.ok(originalCompletion <= consumedAt + 2);
    assert.ok(Date.now() - originalCompletion >= 240);
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
