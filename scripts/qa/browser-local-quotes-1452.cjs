#!/usr/bin/env node
// Controlled real-browser UI/transport receipt, not production backend/deployment evidence.
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const os = require("node:os");
const { spawnSync } = require("node:child_process");
const { createHash } = require("node:crypto");
const { createRequire } = require("node:module");
const root = path.resolve(__dirname, "../..");
const frontendRequire = createRequire(
  path.join(root, "frontend-dapp/package.json"),
);
const { chromium } = frontendRequire("@playwright/test");
const bech32 = frontendRequire("bech32");
const addr = (n) => bech32.encode("terra", bech32.toWords(Buffer.alloc(20, n)));
const from = addr(1),
  to = addr(2),
  pairAddr = addr(3);
// Ask the OS for unused ports; a PID-derived number can collide with active ephemeral connections.
let api, origin, providerUrl;
let providerRequests = 0;
const provider = http.createServer((req, res) => {
  providerRequests++;
  res.statusCode = 500;
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.end(
    JSON.stringify({ error: "RPC forbidden during local quote verification" }),
  );
});
const assets = [from, to].map((contract_addr, i) => ({
  id: i + 1,
  symbol: i ? "BBB" : "AAA",
  name: `Synthetic ${i ? "B" : "A"}`,
  contract_addr,
  denom: null,
  decimals: 6,
}));
const pair = {
  pair_address: pairAddr,
  asset_0: assets[0],
  asset_1: assets[1],
  lp_token: addr(4),
  fee_bps: 30,
  is_active: true,
  liquidity_usd: "10000",
};
const tokens = assets.map((a) => ({
  ...a,
  contract_address: a.contract_addr,
  is_cw20: true,
  logo_url: null,
  coingecko_id: null,
  cmc_id: null,
}));
let quoteRequests = 0;
let completedQuoteRequests = 0;
const geometryObservations = [];
// Owned geometry scenarios change transport outcomes, never production budgets.
let geometryFixture = { mode: "success", delayMs: 0, pauseProducer: false };
// Independent fixture ingestion owns freshness; quote requests never renew it.
let observedAtMs = Date.now();
const producerTimer = setInterval(() => {
  if (!geometryFixture.pauseProducer) observedAtMs = Date.now();
}, 4000);
producerTimer.unref();
function quote(q) {
  quoteRequests++;
  const amount = q.get("amount_in") || "1000000";
  const maxSpread = q.get("max_spread") || "0.05";
  const quoteRelative = q.get("protection_policy") === "quote_relative_v1";
  const scale = 10n ** 18n;
  const tolerance = BigInt((maxSpread.split(".")[1] || "").padEnd(18, "0"));
  const floor = (
    (BigInt(amount) * (scale - tolerance) + (quoteRelative ? scale - 1n : 0n)) /
    scale
  ).toString();
  // This fixture has zero market spread and fees. Independently derive the
  // router ceiling while binding the exact UI tolerance to net floors (#1473).
  const ceiling =
    (tolerance * scale + (scale - tolerance) - 1n) / (scale - tolerance);
  const marketCeiling =
    ceiling >= scale
      ? "1"
      : `0.${ceiling.toString().padStart(18, "0")}`
          .replace(/0+$/, "")
          .replace(/\.$/, "");
  const hop = {
    pair: pairAddr,
    offer_token: q.get("token_in"),
    ask_token: q.get("token_out"),
    offer_amount: amount,
    return_amount: amount,
    pool_return_amount: amount,
    book_return_net: "0",
    pool_input_actual: amount,
    book_offer_consumed: "0",
    spread_amount: "0",
    commission_amount: "0",
    min_return: floor,
    declared_hybrid: null,
  };
  return {
    token_in: hop.offer_token,
    token_out: hop.ask_token,
    hops: [hop],
    intermediate_tokens: [hop.offer_token, hop.ask_token],
    quote_kind: "indexer_pool_db",
    estimated_amount_out: amount,
    spot_amount_out: amount,
    tax_kind: "none",
    router_operations: [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: hop.offer_token } },
          ask_asset_info: { token: { contract_addr: hop.ask_token } },
          min_return: floor,
        },
      },
    ],
    execution_metadata: {
      version: 1,
      generation: `browser-fixture${observedAtMs}`,
      input_amount: amount,
      max_spread: maxSpread,
      trader: q.get("trader"),
      sender: q.get("sender"),
      observed_at_ms: observedAtMs,
      expires_at_ms: observedAtMs + 20000,
      policy: "pool_only",
      ...(quoteRelative
        ? {
            protection_policy: "quote_relative_v1",
            execution_market_spread: marketCeiling,
          }
        : {}),
      minimum_receive: floor,
      hops: [hop],
    },
  };
}
const server = http.createServer((req, res) => {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Content-Type", "application/json");
  if (req.method === "OPTIONS") {
    res.end();
    return;
  }
  const receivedAt = Date.now();
  const u = new URL(req.url, api);
  if (!u.pathname.startsWith("/api/")) {
    let requested;
    try {
      requested = path.resolve(buildDir, "." + decodeURIComponent(u.pathname));
    } catch {
      res.writeHead(400);
      res.end();
      return;
    }
    if (requested !== buildDir && !requested.startsWith(buildDir + path.sep)) {
      res.writeHead(404);
      res.end();
      return;
    }
    const exists = fs.existsSync(requested) && fs.statSync(requested).isFile();
    const file = exists ? requested : path.join(buildDir, "index.html");
    if ((!exists && path.extname(u.pathname)) || !fs.existsSync(file)) {
      res.writeHead(404);
      res.end();
      return;
    }
    const mime = {
      ".html": "text/html",
      ".js": "text/javascript",
      ".css": "text/css",
      ".svg": "image/svg+xml",
      ".png": "image/png",
      ".ico": "image/x-icon",
      ".woff": "font/woff",
      ".woff2": "font/woff2",
      ".json": "application/json",
    };
    res.setHeader(
      "Content-Type",
      mime[path.extname(file)] || "application/octet-stream",
    );
    fs.createReadStream(file).pipe(res);
    return;
  }
  let body;
  if (u.pathname === "/api/v1/route/solve") {
    if (geometryFixture.mode === "success") body = quote(u.searchParams);
    else {
      quoteRequests++;
      res.statusCode = geometryFixture.mode === "rejection" ? 422 : 503;
      body = {
        error:
          geometryFixture.mode === "rejection"
            ? "unsupported_semantics"
            : geometryFixture.mode === "missing"
              ? "missing_observations"
              : "stale_observations",
        generation: `browser-fixture${observedAtMs}`,
      };
    }
    geometryObservations.push({
      amount_in: u.searchParams.get("amount_in"),
      status: res.statusCode,
      observed_at_ms: body.execution_metadata?.observed_at_ms ?? null,
      expires_at_ms: body.execution_metadata?.expires_at_ms ?? null,
      received_at_ms: receivedAt,
    });
    if (geometryObservations.length > 32) geometryObservations.shift();
  } else if (u.pathname === "/api/v1/pairs")
    body = { items: [pair], total: 1, limit: 100, offset: 0 };
  else if (u.pathname === "/api/v1/tokens") body = tokens;
  else if (u.pathname === `/api/v1/pairs/${pairAddr}`) body = pair;
  else if (/candles|trades/.test(u.pathname)) body = [];
  else if (/limit-book/.test(u.pathname))
    body = {
      bids: [],
      asks: [],
      orders: [],
      total: 0,
      has_more: false,
      next_cursor: null,
    };
  else if (/order-book-head/.test(u.pathname)) body = { bid: null, ask: null };
  else if (/oracle/.test(u.pathname))
    body = {
      price: "1",
      symbol: "USTC",
      source: "fixture",
      updated_at: new Date().toISOString(),
    };
  else if (/progress/.test(u.pathname))
    body = {
      stage: "idle",
      done: 0,
      total: 0,
      label: "",
      cache_hit: true,
      updated_at_ms: Date.now(),
      quote_generation: `browser-fixture${observedAtMs}`,
      quote_eligibility:
        geometryFixture.mode === "success" ? "executable_quote" : "waiting",
    };
  else body = {};
  res.setHeader("X-Fixture-Received-At", String(receivedAt));
  res.setHeader("X-Fixture-Sent-At", String(Date.now()));
  const responseBody = JSON.stringify(body);
  const delay =
    u.pathname === "/api/v1/route/solve" ? geometryFixture.delayMs : 0;
  const send = () => {
    if (!res.destroyed) res.end(responseBody);
    if (u.pathname === "/api/v1/route/solve") completedQuoteRequests++;
  };
  if (delay)
    setTimeout(() => {
      send();
    }, delay);
  else send();
});
const buildDir = fs.mkdtempSync(
  path.join(os.tmpdir(), "cl8y-local-quotes-1452-"),
);
let browser;
let buildProvenance;
async function main() {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  api = origin = `http://127.0.0.1:${server.address().port}`;
  await new Promise((resolve, reject) => {
    provider.once("error", reject);
    provider.listen(0, "127.0.0.1", resolve);
  });
  providerUrl = `http://127.0.0.1:${provider.address().port}`;
  const fixtureEnv = {
    ...process.env,
    VITE_FACTORY_ADDRESS: addr(5),
    VITE_ROUTER_ADDRESS: addr(6),
    VITE_INDEXER_URL: api,
    VITE_TERRA_LCD_URL: providerUrl,
    VITE_TERRA_RPC_URL: providerUrl,
    VITE_WC_PROJECT_ID: "controlled-fixture",
    VITE_DEV_MODE: "false",
  };
  buildProvenance = {
    source_revision: spawnSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).stdout.trim(),
    source_dirty: !!spawnSync("git", ["status", "--porcelain"], {
      cwd: root,
      encoding: "utf8",
    }).stdout.trim(),
    built_at_utc: new Date().toISOString(),
  };
  const built = spawnSync(
    "bash",
    [
      "scripts/with-node.sh",
      "--cwd",
      "frontend-dapp",
      "--",
      "npm",
      "run",
      "build",
      "--",
      "--outDir",
      buildDir,
    ],
    { cwd: root, env: fixtureEnv, encoding: "utf8" },
  );
  if (built.status !== 0)
    throw new Error(
      `Production fixture build failed: ${built.stdout.slice(-1500)} ${built.stderr.slice(-1500)}`,
    );
  buildProvenance.served_assets = fs
    .readdirSync(path.join(buildDir, "assets"))
    .filter((name) => name.endsWith(".js"))
    .sort()
    .map((name) => ({
      path: `/assets/${name}`,
      sha256: createHash("sha256")
        .update(fs.readFileSync(path.join(buildDir, "assets", name)))
        .digest("hex"),
    }));
  const launchFlags = [
    "--no-sandbox",
    "--disable-gpu",
    "--disable-background-timer-throttling",
    "--disable-renderer-backgrounding",
    "--disable-backgrounding-occluded-windows",
  ];
  let sharedMemoryMode = "platform_default";
  if (process.platform === "linux") {
    try {
      const shm = fs.statfsSync("/dev/shm");
      if (shm.bavail * shm.bsize >= 256 * 1024 * 1024)
        sharedMemoryMode = "native_shared_memory";
      else {
        sharedMemoryMode = "small_shared_memory_fallback";
        launchFlags.push("--disable-dev-shm-usage");
      }
    } catch {
      sharedMemoryMode = "unavailable_shared_memory_fallback";
      launchFlags.push("--disable-dev-shm-usage");
    }
  }
  let browserVersion;
  const receipts = [];
  for (const viewport of process.env.LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY
    ? []
    : [
        { width: 1440, height: 1000 },
        { width: 390, height: 844 },
      ]) {
    for (const surface of ["Swap", "Trade"]) {
      const initializingAt = Date.now();
      browser = await chromium.launch({
        channel: "chrome",
        headless: true,
        args: launchFlags,
      });
      browserVersion = browser.version();
      const context = await browser.newContext({ viewport });
      const blocked = [];
      const errors = [];
      // Observe passively: allowed API transport has no Playwright interception.
      context.on("request", (request) => {
        if (
          ["fetch", "xhr"].includes(request.resourceType()) &&
          !request.url().startsWith(api) &&
          !request.url().startsWith(origin)
        )
          blocked.push(request.url());
      });
      await context.addInitScript(() => {
        window.__quoteFetchTrace = [];
        window.__quoteLongTasks = [];
        window.__quoteResources = [];
        new PerformanceObserver((list) => {
          for (const e of list.getEntries())
            if (e.name.includes("/route/solve?"))
              window.__quoteResources.push({
                name: e.name,
                start: e.startTime,
                fetch_start: e.fetchStart,
                request_start: e.requestStart,
                response_start: e.responseStart,
                response_end: e.responseEnd,
                duration: e.duration,
                connect_start: e.connectStart,
                connect_end: e.connectEnd,
                transfer_size: e.transferSize,
              });
        }).observe({ type: "resource", buffered: true });
        const original = window.fetch.bind(window);
        window.fetch = async (...args) => {
          const url = String(args[0]);
          const start = performance.now();
          const response = await original(...args);
          if (url.includes("/route/solve?"))
            window.__quoteFetchTrace.push({
              url,
              start,
              end: performance.now(),
              server_received_at: Number(
                response.headers.get("X-Fixture-Received-At"),
              ),
              server_sent_at: Number(response.headers.get("X-Fixture-Sent-At")),
              time_origin: performance.timeOrigin,
            });
          return response;
        };
        new PerformanceObserver((list) => {
          for (const e of list.getEntries())
            window.__quoteLongTasks.push({
              start: e.startTime,
              duration: e.duration,
            });
        }).observe({ entryTypes: ["longtask"] });
      });
      const page = await context.newPage();
      await page.bringToFront();
      page.on("pageerror", (e) => errors.push(e.message));
      const url =
        surface === "Swap"
          ? `${origin}/?pay=${from}&receive=${to}`
          : `${origin}/trade/${pairAddr}`;
      await page.goto(url);
      const input = page
        .getByTestId(
          surface === "Swap"
            ? "swap-you-pay-amount"
            : "limit-order-escrow-amount-input",
        )
        .first();
      await input.waitFor({ timeout: 20000 });
      const risk = page.getByRole("checkbox", {
        name: /I understand these risks/,
      });
      if (await risk.isVisible()) {
        await risk.check();
        await page.getByRole("button", { name: "Continue to the app" }).click();
      }
      // Metadata/process initialization is excluded. Warm every loaded JS path before measurements.
      await input.fill("1");
      const result = page
        .getByTestId(
          surface === "Swap"
            ? "swap-you-receive"
            : "trade-market-expected-receive",
        )
        .first();
      if (surface === "Swap")
        await page.waitForFunction(
          () =>
            document.querySelector('[data-testid="swap-you-receive"]')
              ?.value === "1",
        );
      else
        await page.waitForFunction(() =>
          document
            .querySelector('[data-testid="trade-market-expected-receive"]')
            ?.textContent?.trim()
            .startsWith("1."),
        );
      await page.evaluate(() => document.fonts.ready);
      await page.waitForLoadState("networkidle");
      const initializationMs = Date.now() - initializingAt;
      let profiler;
      if (
        surface === "Trade" &&
        viewport.width === 390 &&
        process.env.LOCAL_QUOTE_BROWSER_PROFILE
      ) {
        profiler = await context.newCDPSession(page);
        await profiler.send("Profiler.enable");
        await profiler.send("Profiler.start");
      }
      const initialBlocked = blocked.length;
      const samples = [];
      const traces = [];
      for (let i = 2; i <= 11; i++) {
        await page.bringToFront();
        const amount = String(i);
        const ids = {
          inputId:
            surface === "Swap"
              ? "swap-you-pay-amount"
              : "limit-order-escrow-amount-input",
          resultId:
            surface === "Swap"
              ? "swap-you-receive"
              : "trade-market-expected-receive",
          amount,
          surface,
        };
        await page.evaluate(({ inputId, resultId, amount, surface }) => {
          if (document.visibilityState !== "visible")
            throw new Error("Benchmark requires a visible foreground page");
          const input = document.querySelector(`[data-testid="${inputId}"]`);
          const state = (window.__quoteMeasurement = {
            start: null,
            end: null,
          });
          const check = () => {
            const el = document.querySelector(`[data-testid="${resultId}"]`);
            const correct =
              surface === "Swap"
                ? el?.value === amount
                : el?.textContent?.trim().startsWith(`${amount}.`);
            if (state.start !== null && state.end === null && correct)
              state.end = performance.now();
          };
          const onInput = (event) => {
            if (
              event.target.matches?.(`[data-testid="${inputId}"]`) &&
              state.start === null
            ) {
              state.start = performance.now();
              check();
            }
          };
          document.addEventListener("input", onInput, true);
          const observer = new MutationObserver(check);
          observer.observe(document.body, {
            subtree: true,
            childList: true,
            attributes: true,
            characterData: true,
          });
          const timer = setInterval(check, 5);
          state.cleanup = () => {
            observer.disconnect();
            clearInterval(timer);
            document.removeEventListener("input", onInput, true);
          };
        }, ids);
        // Exercise Chromium's actual input event through Playwright, rather than emulating React's value tracker.
        await input.fill(amount);
        try {
          await page.waitForFunction(
            () => window.__quoteMeasurement?.end !== null,
            undefined,
            { polling: 10, timeout: 5000 },
          );
        } catch (error) {
          process.stderr.write(
            JSON.stringify({
              surface,
              viewport,
              amount,
              quoteRequests,
              blocked,
              errors,
              url: page.url(),
              input: await input.inputValue(),
              result:
                surface === "Swap"
                  ? await result.inputValue()
                  : await result.textContent(),
              trace: await page.evaluate(() => window.__quoteFetchTrace),
            }) + "\n",
          );
          throw error;
        }
        const ms = await page.evaluate(() => {
          const state = window.__quoteMeasurement;
          state.cleanup();
          return {
            duration: state.end - state.start,
            visibility: document.visibilityState,
            fetch: window.__quoteFetchTrace.filter(
              (t) => t.start >= state.start,
            ),
            resources: window.__quoteResources.filter(
              (t) => t.start >= state.start,
            ),
            long_tasks: window.__quoteLongTasks.filter(
              (t) => t.start >= state.start,
            ),
          };
        });
        samples.push(Number(ms.duration.toFixed(3)));
        traces.push(ms);
      }
      if (profiler) {
        const { profile } = await profiler.send("Profiler.stop");
        fs.writeFileSync(
          process.env.LOCAL_QUOTE_BROWSER_PROFILE,
          JSON.stringify(profile),
        );
      }
      const sorted = [...samples].sort((a, b) => a - b);
      receipts.push({
        surface,
        viewport,
        initialization_ms: initializationMs,
        samples_ms: samples,
        traces,
        p50_ms: sorted[4],
        p95_ms: sorted[9],
        max_ms: sorted[9],
        initial_blocked_chain_requests: initialBlocked,
        quote_triggered_chain_requests: blocked.length - initialBlocked,
        blocked_chain_urls: blocked,
        page_errors: errors,
      });
      await context.close();
      await browser.close();
      browser = undefined;
    }
  }
  let geometry = [];
  if (
    process.env.LOCAL_QUOTE_BROWSER_GEOMETRY ||
    process.env.LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY
  ) {
    browser = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: launchFlags,
    });
    browserVersion = browser.version();
    const { verifySwapGeometry } = require("./swap-layout-geometry-1498.cjs");
    geometry = await verifySwapGeometry({
      browser,
      origin,
      from,
      to,
      fixture: {
        set(options = {}) {
          geometryFixture = {
            mode: "success",
            delayMs: 0,
            pauseProducer: false,
            ...options,
          };
          observedAtMs = Date.now() - (options.observedAgeMs || 0);
        },
        requests: () => quoteRequests,
        completed: () => completedQuoteRequests,
        observations: () => [...geometryObservations],
      },
    });
    await browser.close();
    browser = undefined;
  }
  const result = {
    recorded_at: new Date().toISOString(),
    environment:
      "Full Google Chrome in headless foreground mode, production Vite build served over native local HTTP with synthetic indexer; each case owns an independent browser process; initialization/fonts/network idle excluded and measured separately; wallet disconnected; no broadcast; not production reference evidence",
    measurement:
      "Browser input event capture to useful DOM result; document MutationObserver and 5 ms fallback; foreground page; native HTTP transport; risk acknowledgement accepted before timing",
    browser_channel: "chrome",
    browser_version: browserVersion,
    launch_flags: launchFlags,
    shared_memory_mode: sharedMemoryMode,
    quote_requests: quoteRequests,
    provider_requests: providerRequests,
    receipts,
    geometry,
    build_provenance: buildProvenance,
    geometry_measurement: geometry.length
      ? "Owned production-build bounding rectangles after initialized quote; independent browser contexts, foreground, reduced motion and 200% root text. Timing trials are separate and retain original input clocks."
      : undefined,
  };
  const output =
    process.env.LOCAL_QUOTE_BROWSER_RECEIPT ||
    path.join(root, "docs/qa/issue-1452/browser-controlled.json");
  fs.writeFileSync(output, JSON.stringify(result, null, 2) + "\n");
  process.stdout.write(
    JSON.stringify(
      {
        ...result,
        receipts: result.receipts.map(({ traces, ...rest }) => rest),
      },
      null,
      2,
    ) + "\n",
  );
  if (
    providerRequests !== 0 ||
    geometry.some((r) => r.verdict !== "PASS") ||
    receipts.some(
      (r) =>
        r.max_ms >= 250 ||
        r.initial_blocked_chain_requests !== 0 ||
        r.quote_triggered_chain_requests !== 0 ||
        r.page_errors.length,
    )
  )
    throw new Error("Controlled browser quote acceptance failed; see receipt");
}
main()
  .catch((e) => {
    process.stderr.write(e.stack + "\n");
    process.exitCode = 1;
  })
  .finally(async () => {
    clearInterval(producerTimer);
    if (browser) await browser.close();
    await new Promise((r) => server.close(r));
    await new Promise((r) => provider.close(r));
    fs.rmSync(buildDir, { recursive: true, force: true });
  });
