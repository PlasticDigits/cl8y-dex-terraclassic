const { test } = require("node:test"),
  assert = require("node:assert/strict"),
  http = require("node:http"),
  path = require("node:path");
const { createRequire } = require("node:module"),
  rf = createRequire(
    path.resolve(__dirname, "../../frontend-dapp/package.json"),
  );
const { JSDOM } = rf("jsdom"),
  { chromium } = rf("@playwright/test");
const {
  installReadGuard,
  nativeBlockRules,
} = require("./live-read-guard-1499.cjs");
function harness() {
  const dom = new JSDOM("<form></form>", {
      url: "https://dex.cl8y.com",
      runScripts: "outside-only",
    }),
    w = dom.window,
    calls = [];
  w.fetch = async (input, opts) => {
    calls.push({ input, opts });
    return { ok: true };
  };
  w.XMLHttpRequest.prototype.open = function (...args) {
    this.args = args;
  };
  w.XMLHttpRequest.prototype.send = function () {
    calls.push({ xhr: true });
  };
  w.eval(
    "(" +
      installReadGuard.toString() +
      ")(" +
      JSON.stringify({
        frontend: "https://dex.cl8y.com",
        backend: "https://indexer.dex.cl8y.com",
        remaining: 2,
      }) +
      ")",
  );
  return { w, calls, close: () => dom.window.close() };
}
test("methods, provider paths, actors and unexpected origins are rejected before fetch/XHR transport", async () => {
  const h = harness();
  for (const [url, method] of [
    ["https://indexer.dex.cl8y.com/api/v1/route/solve", "POST"],
    ["https://dex.cl8y.com/cosmos/tx/v1beta1/txs", "GET"],
    ["https://indexer.dex.cl8y.com/api/v1/route/solve?trader=synthetic", "GET"],
    ["https://other.example/api", "GET"],
  ])
    await assert.rejects(h.w.fetch(url, { method }), /Read-only/);
  const xhr = new h.w.XMLHttpRequest();
  assert.throws(() => xhr.open("POST", "https://dex.cl8y.com/tx"), /Read-only/);
  assert.equal(h.calls.length, 0);
  h.close();
});
test("read budget is enforced synchronously without Node roundtrips and redirects cannot be followed", async () => {
  const h = harness(),
    url = "https://indexer.dex.cl8y.com/api/v1/route/solve";
  await h.w.fetch(url);
  await h.w.fetch(url + "/progress");
  await assert.rejects(h.w.fetch(url), /Read-only/);
  assert.equal(h.calls.length, 2);
  assert.equal(h.calls[0].opts.redirect, "error");
  assert.equal(h.w.__liveReadGuard1499.remaining, 0);
  assert.equal(h.w.__liveReadGuard1499.rejected, 1);
  h.close();
});
test("workers, sockets, beacons and direct form submission cannot create unguarded writes", () => {
  const h = harness();
  for (const name of ["Worker", "SharedWorker", "WebSocket", "EventSource"])
    assert.throws(() => new h.w[name]("https://dex.cl8y.com/"), /Read-only/);
  assert.equal(
    h.w.navigator.sendBeacon("https://dex.cl8y.com/tx", "synthetic"),
    false,
  );
  assert.throws(() => h.w.document.querySelector("form").submit(), /Read-only/);
  assert.equal(h.calls.length, 0);
  h.close();
});
test("native CDP ordered allow/deny rules preserve public GET while blocking provider paths and redirect escapes", async () => {
  let safeReads = 0,
    providerReads = 0,
    escapeReads = 0;
  const server = http.createServer((req, res) => {
    if (req.url.startsWith("/cosmos/")) providerReads++;
    else if (req.url.startsWith("/escape")) escapeReads++;
    else safeReads++;
    if (req.url === "/redirect") {
      res.writeHead(302, {
        Location: "http://127.0.0.1:" + server.address().port + "/escape",
      });
      res.end();
    } else {
      res.setHeader("Content-Type", "application/json");
      res.end('{"ok":true}');
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const origin = "http://localhost:" + server.address().port;
  let browser;
  try {
    browser = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
    });
    const context = await browser.newContext(),
      page = await context.newPage(),
      cdp = await context.newCDPSession(page);
    await cdp.send("Network.enable");
    await cdp.send("Network.setBlockedURLs", nativeBlockRules(origin, origin));
    await page.goto(origin + "/");
    const results = await page.evaluate(async (origin) => {
      const check = async (p) => {
        try {
          return (await fetch(origin + p)).ok;
        } catch {
          return false;
        }
      };
      return {
        safe: await check("/safe"),
        provider: await check("/cosmos/tx"),
        redirect: await check("/redirect"),
      };
    }, origin);
    assert.deepEqual(results, { safe: true, provider: false, redirect: false });
    assert.equal(providerReads, 0);
    assert.equal(escapeReads, 0);
    assert.ok(safeReads >= 2);
    await context.close();
  } finally {
    if (browser) await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
});
