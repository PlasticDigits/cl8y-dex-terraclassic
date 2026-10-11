#!/usr/bin/env node
// Owned deterministic HTTP/browser fixtures; never production economics or deployment acceptance.
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const { createHash } = require("node:crypto");
const http = require("node:http");
const { spawnSync } = require("node:child_process");
const { createRequire } = require("node:module");
const root = path.resolve(__dirname, "../..");
const requireFrontend = createRequire(
  path.join(root, "frontend-dapp/package.json"),
);
const { chromium } = requireFrontend("@playwright/test");
// Temporary explicit user override; historical 500 ms receipts retain their verdict.
const BROWSER_LATENCY_BUDGET_MS = 750;
// Match the repository's owned Linux browser provisioning. Keep these explicit
// in receipts; graphics setup does not change the quote or latency acceptance.
const browserLaunchFlags = [
  "--no-sandbox",
  "--disable-gpu",
  "--disable-background-timer-throttling",
  "--disable-renderer-backgrounding",
];
let sharedMemoryMode = "platform_default";
if (process.platform === "linux") {
  try {
    const shm = fs.statfsSync("/dev/shm");
    if (shm.bavail * shm.bsize >= 256 * 1024 * 1024)
      sharedMemoryMode = "native_shared_memory";
    else {
      sharedMemoryMode = "small_shared_memory_fallback";
      browserLaunchFlags.push("--disable-dev-shm-usage");
    }
  } catch {
    sharedMemoryMode = "unavailable_shared_memory_fallback";
    browserLaunchFlags.push("--disable-dev-shm-usage");
  }
}
const bech32 = requireFrontend("bech32");
const address = (n) =>
  bech32.encode("terra", bech32.toWords(Buffer.alloc(20, n)));
const matrix = JSON.parse(
  fs.readFileSync(path.join(root, "docs/qa/issue-1473/matrix.json"), "utf8"),
);
// Unregistered symbols/decimals prove pricing is not special-cased to product tokens.
const corpus = {
  ...matrix.assets,
  CUSTOM9: { identity: address(41), decimals: 9 },
  CUSTOM12: { identity: address(42), decimals: 12 },
  cLUNC: { identity: address(43), decimals: 6 },
};
const pairLabels = [...matrix.pairs, ["CUSTOM9", "CUSTOM12"]];
const asset = (label) => ({
  symbol: label,
  name: label,
  decimals: corpus[label].decimals,
  contract_addr: corpus[label].identity.startsWith("terra1")
    ? corpus[label].identity
    : null,
  denom: corpus[label].identity.startsWith("terra1")
    ? null
    : corpus[label].identity,
});
const wrappedLabel = (label) =>
  label === "USTC" ? "cUSTC" : label === "LUNC" ? "cLUNC" : label;
const mappedIdentity = (id) =>
  id === "uusd"
    ? corpus.cUSTC.identity
    : id === "uluna"
      ? corpus.cLUNC.identity
      : id;
const pairs = [],
  factoryKeys = new Set();
for (const [first, second] of pairLabels) {
  const a = wrappedLabel(first),
    b = wrappedLabel(second);
  const key = [corpus[a].identity, corpus[b].identity].sort().join(":");
  if (factoryKeys.has(key)) continue;
  factoryKeys.add(key);
  const i = pairs.length;
  pairs.push({
    pair_address: address(i + 60),
    lp_token: address(i + 80),
    asset_0: asset(a),
    asset_1: asset(b),
    fee_bps: 30,
    is_active: true,
    liquidity_usd: "2500",
  });
}
const tokens = Object.keys(corpus).map((label) => ({
  ...asset(label),
  contract_address: asset(label).contract_addr,
  is_cw20: !!asset(label).contract_addr,
  logo_url: null,
  coingecko_id: null,
  cmc_id: null,
}));
const identities = new Map(
  Object.entries(corpus).map(([label, a]) => [a.identity, { ...a, label }]),
);
const power = (d) => 10n ** BigInt(d);
function raw(human, decimals) {
  const [whole, fraction = ""] = human.split(".");
  if (fraction.length > decimals)
    throw new Error("Unrepresentable fixture input");
  return (
    BigInt(whole) * power(decimals) +
    BigInt(fraction.padEnd(decimals, "0") || "0")
  );
}
function human(value, decimals) {
  const digits = value.toString().padStart(decimals + 1, "0");
  if (!decimals) return digits;
  return `${digits.slice(0, -decimals)}.${digits.slice(-decimals)}`
    .replace(/0+$/, "")
    .replace(/\.$/, "");
}
function economics(from, to, input, spread, quoteRelative = true) {
  const a = identities.get(from),
    b = identities.get(to);
  // Pinned reserves: 1000 UST1/first leg, 1500 CL8Y/second leg; direction inverses reuse exact same reserves.
  const pair = pairs.find(
    (p) =>
      [p.asset_0, p.asset_1].some(
        (a) => (a.contract_addr || a.denom) === mappedIdentity(from),
      ) &&
      [p.asset_0, p.asset_1].some(
        (a) => (a.contract_addr || a.denom) === mappedIdentity(to),
      ),
  );
  if (!a || !b || !pair) return { reason: "no_route" };
  const forward =
    (pair.asset_0.contract_addr || pair.asset_0.denom) === mappedIdentity(from);
  const reserveIn = BigInt(forward ? 1000 : 1500) * power(a.decimals);
  const reserveOut = BigInt(forward ? 1500 : 1000) * power(b.decimals);
  const x = BigInt(input),
    gross = (x * reserveOut) / (reserveIn + x);
  const commission = (gross * 30n) / 10000n,
    output = gross - commission;
  const spot = (x * reserveOut) / reserveIn;
  const spreadRaw = spot - gross;
  const maxBps = BigInt(Math.round(Number(spread) * 10000));
  const roundedFloor =
    (output * (10000n - maxBps) + (quoteRelative ? 9999n : 0n)) / 10000n;
  const floor = output > 0n && roundedFloor === 0n ? 1n : roundedFloor;
  const reason =
    output <= 0n
      ? "amount_too_small"
      : !quoteRelative && spreadRaw * 10000n > gross * maxBps
        ? "protection"
        : undefined;
  const scale = 10n ** 18n,
    tolerance = (maxBps * scale) / 10000n;
  const numerator = spreadRaw > gross ? gross : spreadRaw;
  const denominator = gross * (scale - tolerance);
  const ceilingNumerator = (numerator * scale + gross * tolerance) * scale;
  const ceiling =
    gross === 0n ? 0n : (ceilingNumerator + denominator - 1n) / denominator;
  const executionMarketSpread = human(ceiling > scale ? scale : ceiling, 18);
  return {
    pair,
    output,
    spot,
    gross,
    spreadRaw,
    commission,
    floor,
    reason,
    executionMarketSpread,
  };
}
// Independently derive integer transition boundaries; test their neighboring
// raw inputs rather than replacing a rejected size with an ordinary small one.
function boundaryAmounts(from, to, spread) {
  const decimals = identities.get(from).decimals;
  function first(predicate) {
    let lo = 1n,
      hi = 3000n * power(decimals);
    while (lo < hi) {
      const mid = (lo + hi) / 2n;
      if (predicate(economics(from, to, mid.toString(), spread))) hi = mid;
      else lo = mid + 1n;
    }
    return lo;
  }
  const dust = first((result) => result.output > 0n);
  const netTwo = first((result) => result.output >= 2n),
    netNineteen = first((result) => result.output >= 19n);
  const bps = BigInt(Math.round(Number(spread) * 10000));
  const protection = first(
    (result) => result.spreadRaw * 10000n > result.gross * bps,
  );
  return [dust, netTwo, netNineteen, protection]
    .flatMap((boundary) => [boundary - 1n, boundary, boundary + 1n])
    .filter((value) => value > 0n)
    .map((value) => human(value, decimals));
}
let observedAt = Date.now(),
  generation = 1;
// Observations have an independent schedule; requests never renew timestamps.
const publisher = setInterval(() => {
  observedAt = Date.now();
  generation++;
}, 4000);
const cache = new Set();
let quoteRequests = 0,
  providerRequests = 0,
  browser;
const buildDir = fs.mkdtempSync(path.join(os.tmpdir(), "cl8y-trading1473-"));
let origin;
let sourceRevisionAtBuild, sourceDirtyAtBuild;
let currentStage = "startup",
  verificationFinished = false;
const requestSamples = [];
const provider = http.createServer((_req, res) => {
  providerRequests++;
  res.writeHead(500);
  res.end("{}");
});
function quote(q) {
  quoteRequests++;
  const from = q.get("token_in"),
    to = q.get("token_out"),
    input = q.get("amount_in"),
    spread = q.get("max_spread") || "0.05";
  const result = economics(
    from,
    to,
    input,
    spread,
    q.get("protection_policy") === "quote_relative_v1",
  );
  const key = JSON.stringify([
    from,
    to,
    input,
    spread,
    q.get("protection_policy"),
    generation,
  ]);
  const hit = cache.has(key);
  cache.add(key);
  const common = {
    generation: `browser1473:${generation}`,
    observed_at_ms: observedAt,
    expires_at_ms: observedAt + 20000,
  };
  if (result.reason)
    return {
      status: 422,
      hit,
      body: {
        error: result.reason,
        ...common,
        execution_supported: false,
        ...(result.reason === "protection"
          ? { indicative_amount_out: result.output.toString() }
          : {}),
      },
    };
  const h = {
    pair: result.pair.pair_address,
    offer_token: mappedIdentity(from),
    ask_token: mappedIdentity(to),
    offer_amount: input,
    return_amount: result.output.toString(),
    pool_return_amount: result.output.toString(),
    pool_input_actual: input,
    book_offer_consumed: "0",
    book_return_net: "0",
    spread_amount: result.spreadRaw.toString(),
    commission_amount: result.commission.toString(),
    min_return: result.floor.toString(),
    declared_hybrid: null,
  };
  const info = (id) =>
    id.startsWith("terra1")
      ? { token: { contract_addr: id } }
      : { native_token: { denom: id } };
  return {
    status: 200,
    hit,
    body: {
      token_in: from,
      token_out: to,
      hops: [h],
      intermediate_tokens: [mappedIdentity(from), mappedIdentity(to)],
      quote_kind: "indexer_pool_db",
      tax_kind: "none",
      estimated_amount_out: h.return_amount,
      spot_amount_out: result.spot.toString(),
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: info(mappedIdentity(from)),
            ask_asset_info: info(mappedIdentity(to)),
            min_return: h.min_return,
          },
        },
      ],
      execution_metadata: {
        version: 1,
        ...common,
        input_amount: input,
        max_spread: spread,
        trader: q.get("trader"),
        sender: q.get("sender"),
        policy: "pool_only",
        ...(from === "uusd" ||
        from === "uluna" ||
        to === "uusd" ||
        to === "uluna"
          ? {
              wrapper: {
                input_token: from,
                output_token: to,
                input_amount: input,
                route_input_amount: input,
                route_output_amount: h.return_amount,
                minimum_receive_base: h.min_return,
                wrap_input: !from.startsWith("terra1"),
                unwrap_output: !to.startsWith("terra1"),
                mapper: address(52),
                treasury: address(53),
                floor_protocol_version: 1,
              },
            }
          : {}),
        ...(q.get("protection_policy") === "quote_relative_v1"
          ? {
              protection_policy: "quote_relative_v1",
              execution_market_spread: result.executionMarketSpread,
            }
          : {}),
        minimum_receive: h.min_return,
        hops: [h],
      },
    },
  };
}
const mime = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".json": "application/json",
};
const server = http.createServer((req, res) => {
  res.setHeader("X-Fixture-Received-At", String(Date.now()));
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Content-Type", "application/json");
  if (req.method === "OPTIONS") {
    res.end();
    return;
  }
  const url = new URL(req.url, origin);
  if (!url.pathname.startsWith("/api/")) {
    let requested;
    try {
      requested = path.resolve(
        buildDir,
        "." + decodeURIComponent(url.pathname),
      );
    } catch {
      res.writeHead(400);
      res.end();
      return;
    }
    if (!requested.startsWith(buildDir + path.sep) && requested !== buildDir) {
      res.writeHead(404);
      res.end();
      return;
    }
    const exists = fs.existsSync(requested) && fs.statSync(requested).isFile();
    const file = exists ? requested : path.join(buildDir, "index.html");
    if (!exists && path.extname(url.pathname)) {
      res.writeHead(404);
      res.end();
      return;
    }
    res.setHeader(
      "Content-Type",
      mime[path.extname(file)] || "application/octet-stream",
    );
    fs.createReadStream(file).pipe(res);
    return;
  }
  let body = {};
  if (url.pathname === "/api/v1/route/solve") {
    const response = quote(url.searchParams);
    const metadata = response.body.execution_metadata;
    requestSamples.push({
      from:
        identities.get(url.searchParams.get("token_in"))?.label || "unknown",
      to: identities.get(url.searchParams.get("token_out"))?.label || "unknown",
      input: url.searchParams.get("amount_in"),
      spread: url.searchParams.get("max_spread"),
      request_policy: url.searchParams.get("protection_policy"),
      status: response.status,
      output: response.body.estimated_amount_out,
      floor: metadata?.minimum_receive,
      execution_market_spread: metadata?.execution_market_spread,
      generation: metadata?.generation,
      observed_at_ms: metadata?.observed_at_ms,
      expires_at_ms: metadata?.expires_at_ms,
      wrapper: metadata?.wrapper
        ? {
            wrap_input: metadata.wrapper.wrap_input,
            unwrap_output: metadata.wrapper.unwrap_output,
            route_input_amount: metadata.wrapper.route_input_amount,
            route_output_amount: metadata.wrapper.route_output_amount,
            minimum_receive_base: metadata.wrapper.minimum_receive_base,
          }
        : null,
    });
    if (requestSamples.length > 64) requestSamples.shift();
    res.statusCode = response.status;
    res.setHeader("X-Fixture-Cache", response.hit ? "hit" : "miss");
    body = response.body;
  } else if (/route\/solve\/progress/.test(url.pathname))
    body = {
      stage: "idle",
      done: 0,
      total: 0,
      cache_hit: true,
      quote_generation: `browser1473:${generation}`,
    };
  else if (url.pathname === "/api/v1/pairs")
    body = { items: pairs, total: pairs.length, limit: 100, offset: 0 };
  else if (url.pathname === "/api/v1/tokens") body = tokens;
  else if (/\/pairs\/[^/]+$/.test(url.pathname))
    body = pairs.find((p) => url.pathname.endsWith(p.pair_address)) || {};
  else if (/trades|candles/.test(url.pathname)) body = [];
  else if (/oracle/.test(url.pathname))
    body = { price_usd: "1", ticker: "ustc", sources: [] };
  res.setHeader("X-Fixture-Sent-At", String(Date.now()));
  res.end(JSON.stringify(body));
});
const listen = (server) =>
  new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
const receipts = [],
  failures = [];
async function acknowledge(page) {
  const risk = page.getByRole("checkbox", { name: /I understand these risks/ });
  if (await risk.isVisible()) {
    await risk.check();
    await page.getByRole("button", { name: "Continue to the app" }).click();
  }
}
async function measure(page, amount, expected, reason) {
  if ((await page.getByTestId("swap-you-pay-amount").inputValue()) === amount) {
    await page.getByTestId("swap-you-pay-amount").fill("");
  }
  await page.evaluate(
    ({ expected, reason }) => {
      const input = '[data-testid="swap-you-pay-amount"]';
      const state = (window.__measure1473 = { start: null, end: null });
      const start = (e) => {
        if (e.target.matches?.(input) && state.start === null) {
          state.start = performance.now();
          state.intent = {
            pay: document.querySelector('[aria-label="Select token you pay"]')
              ?.value,
            receive: document.querySelector(
              '[aria-label="Select token you receive"]',
            )?.value,
            amount: e.target.value,
            preset: document
              .querySelector(
                '[data-testid^="swap-slippage-preset-"][aria-pressed="true"]',
              )
              ?.getAttribute("data-testid"),
            custom: document.querySelector(
              '[data-testid="swap-slippage-custom"] input',
            )?.value,
          };
        }
      };
      document.addEventListener("input", start, true);
      const ready = () => {
        const output = document.querySelector(
          '[data-testid="swap-you-receive"]',
        );
        // Poll only the actual output/error surfaces. Reading body.innerText
        // every five milliseconds forces unrelated full-document layout.
        const text = reason
          ? document.querySelector(
              '[data-testid="swap-quote-error"], [data-testid="swap-quote-unavailable"], [data-testid="swap-quote-retry-error"]',
            )?.textContent || ""
          : "";
        return reason === "amount_too_small"
          ? text.includes("too small to receive a positive output") &&
              output?.value === ""
          : reason === "no_route"
            ? output?.value === "" &&
              text.includes("No supported route connects these tokens.")
            : reason === "protection"
              ? output?.value === "" &&
                text.includes("selected price protection") &&
                !document.querySelector(
                  '[data-testid="swap-indicative-estimate"]',
                )
              : output?.value === expected &&
                output?.getAttribute("aria-busy") !== "true";
      };
      const check = () => {
        if (
          state.start === null ||
          state.end !== null ||
          !ready() ||
          state.readyFrame != null
        )
          return;
        // A current DOM value alone is insufficient. Recheck after a complete
        // frame opportunity, without resetting the original input-event clock.
        state.readyFrame = requestAnimationFrame(() => {
          state.paintFrame = requestAnimationFrame(() => {
            const output = document.querySelector(
              '[data-testid="swap-you-receive"]',
            );
            const rect = output?.getBoundingClientRect();
            const style = output ? getComputedStyle(output) : null;
            if (
              ready() &&
              rect?.width > 0 &&
              rect?.height > 0 &&
              style?.display !== "none" &&
              style?.visibility !== "hidden"
            )
              state.end = performance.now();
            else state.readyFrame = null;
          });
        });
      };
      state.frame = requestAnimationFrame(check);
      state.timer = setInterval(check, 5);
      state.cleanup = () => {
        cancelAnimationFrame(state.frame);
        cancelAnimationFrame(state.readyFrame);
        cancelAnimationFrame(state.paintFrame);
        clearInterval(state.timer);
        document.removeEventListener("input", start, true);
      };
    },
    { expected, reason },
  );
  await page.getByTestId("swap-you-pay-amount").fill(amount);
  try {
    await page.waitForFunction(
      () => window.__measure1473?.end != null,
      undefined,
      // Observe late output without treating it as success: every initialized
      // Executable samples above the recorded latency budget remain failures.
      { timeout: 15000, polling: 10 },
    );
    if (
      /route validation (rejected|failed)/i.test(
        await page.locator("body").innerText(),
      )
    )
      throw new Error("Generic validation copy reached browser");
    if (reason) {
      const button = page.getByTestId("swap-submit");
      if (
        (await button.isVisible()) &&
        (await button.isEnabled()) &&
        !/connect wallet/i.test(await button.innerText())
      )
        throw new Error("Blocked quote allowed submission");
    }
    return await page.evaluate(
      () => window.__measure1473.end - window.__measure1473.start,
    );
  } finally {
    await page.evaluate(() => window.__measure1473.cleanup());
  }
}
function stats(samples) {
  const sorted = samples.slice().sort((a, b) => a - b),
    at = (p) => sorted[Math.max(0, Math.ceil(sorted.length * p) - 1)] ?? null;
  return {
    count: sorted.length,
    p50_ms: at(0.5),
    p95_ms: at(0.95),
    p99_ms: at(0.99),
    max_ms: at(1),
  };
}
async function measurementTrace(page) {
  return page.evaluate(() => {
    const state = window.__measure1473;
    const since = (values) =>
      (values || []).filter(
        (value) => value.start >= (state?.start ?? Infinity),
      );
    return {
      start: state?.start,
      end: state?.end,
      intent: state?.intent,
      fetch_attempts: since(window.__browser1473FetchTrace),
      resources: since(window.__browser1473Resources),
      long_tasks: since(window.__browser1473LongTasks),
    };
  });
}
function artifactDigest(directory) {
  const hash = createHash("sha256");
  function add(base) {
    for (const name of fs.readdirSync(base).sort()) {
      const file = path.join(base, name);
      if (fs.statSync(file).isDirectory()) add(file);
      else {
        hash.update(path.relative(directory, file));
        hash.update(fs.readFileSync(file));
      }
    }
  }
  add(directory);
  return hash.digest("hex");
}
async function main() {
  await listen(server);
  origin = `http://127.0.0.1:${server.address().port}`;
  await listen(provider);
  const forbidden = `http://127.0.0.1:${provider.address().port}`;
  currentStage = "build";
  sourceRevisionAtBuild = spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim();
  sourceDirtyAtBuild = !!spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim();
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
    {
      cwd: root,
      env: {
        ...process.env,
        VITE_NETWORK: "mainnet",
        VITE_INDEXER_URL: origin,
        VITE_FACTORY_ADDRESS: address(50),
        VITE_ROUTER_ADDRESS: address(51),
        VITE_CL8Y_TOKEN_ADDRESS: corpus.CL8Y.identity,
        VITE_TERRA_LCD_URL: forbidden,
        VITE_TERRA_RPC_URL: forbidden,
        VITE_WC_PROJECT_ID: "controlled-fixture",
        VITE_DEV_MODE: "false",
        VITE_WRAP_MAPPER_ADDRESS: address(52),
        VITE_TREASURY_ADDRESS: address(53),
        VITE_LUNC_C_TOKEN_ADDRESS: corpus.cLUNC.identity,
        VITE_USTC_C_TOKEN_ADDRESS: corpus.cUSTC.identity,
      },
      encoding: "utf8",
    },
  );
  if (built.status !== 0)
    throw new Error(
      `Fixture build failed: ${built.stdout.slice(-1800)} ${built.stderr.slice(-1000)}`,
    );
  currentStage = "browser-launch";
  browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: browserLaunchFlags,
  });
  for (const viewport of [
    { width: 1440, height: 1000 },
    { width: 390, height: 844 },
  ]) {
    const context = await browser.newContext({ viewport });
    await context.addInitScript(
      ({ labels }) => {
        window.__browser1473FetchTrace = [];
        window.__browser1473Resources = [];
        window.__browser1473LongTasks = [];
        const append = (values, entry) => {
          values.push(entry);
          if (values.length > 64) values.shift();
        };
        const normalized = (rawUrl) => {
          const url = new URL(rawUrl, window.location.href);
          if (!url.pathname.includes("/route/solve")) return null;
          return {
            from: labels[url.searchParams.get("token_in")] || "unknown",
            to: labels[url.searchParams.get("token_out")] || "unknown",
            amount:
              url.searchParams.get("amount_in") ||
              url.searchParams.get("amount_out"),
            spread: url.searchParams.get("max_spread"),
            kind: url.pathname.endsWith("/progress") ? "progress" : "solve",
          };
        };
        const original = window.fetch.bind(window);
        window.fetch = async (...args) => {
          let details;
          try {
            details = normalized(
              typeof args[0] === "string" ? args[0] : args[0].url,
            );
          } catch {
            details = null;
          }
          const start = performance.now();
          try {
            const response = await original(...args);
            if (details)
              append(window.__browser1473FetchTrace, {
                ...details,
                start,
                end: performance.now(),
                status: response.status,
                server_received_at_ms: Number(
                  response.headers.get("X-Fixture-Received-At"),
                ),
                server_sent_at_ms: Number(
                  response.headers.get("X-Fixture-Sent-At"),
                ),
                time_origin_ms: performance.timeOrigin,
              });
            return response;
          } catch (error) {
            if (details)
              append(window.__browser1473FetchTrace, {
                ...details,
                start,
                end: performance.now(),
                error_class: error.name,
              });
            throw error;
          }
        };
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            const details = normalized(entry.name);
            if (details)
              append(window.__browser1473Resources, {
                ...details,
                start: entry.startTime,
                fetch_start: entry.fetchStart,
                request_start: entry.requestStart,
                response_start: entry.responseStart,
                response_end: entry.responseEnd,
                duration: entry.duration,
                connect_start: entry.connectStart,
                connect_end: entry.connectEnd,
                protocol: entry.nextHopProtocol,
              });
          }
        }).observe({ type: "resource", buffered: true });
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries())
            append(window.__browser1473LongTasks, {
              start: entry.startTime,
              duration: entry.duration,
            });
        }).observe({ entryTypes: ["longtask"] });
      },
      {
        labels: Object.fromEntries(
          [...identities].map(([id, asset]) => [id, asset.label]),
        ),
      },
    );
    const page = await context.newPage();
    let externalRequests = 0;
    context.on("request", (request) => {
      if (
        ["fetch", "xhr"].includes(request.resourceType()) &&
        !request.url().startsWith(origin)
      )
        externalRequests++;
    });
    // Defaults are inspected through the same production bundle on both entries.
    for (const entry of ["/", "/trade"]) {
      process.stderr.write(`default ${viewport.width} ${entry}\n`);
      await page.goto(origin + entry);
      await acknowledge(page);
      await page.getByTestId("swap-you-pay-amount").waitFor({ timeout: 20000 });
      if (
        (await page
          .getByRole("combobox", { name: "Select token you pay" })
          .inputValue()) !== "USTC" ||
        (await page
          .getByRole("combobox", { name: "Select token you receive" })
          .inputValue()) !== "CL8Y"
      )
        throw new Error("Native USTC/CL8Y default identity mismatch");
    }
    let scenarios = 0;
    corpusTrials: for (const spread of matrix.max_spreads.filter(
      (spread) =>
        !process.env.BROWSER1473_SPREAD ||
        spread === process.env.BROWSER1473_SPREAD,
    ))
      for (const labels of pairLabels)
        for (const direction of [labels, labels.slice().reverse()]) {
          if (
            process.env.BROWSER1473_LIMIT &&
            scenarios >= Number(process.env.BROWSER1473_LIMIT)
          )
            break corpusTrials;
          const initializationStart = Date.now();
          currentStage = "case-initialize";
          if (
            process.env.BROWSER1473_PAIR &&
            direction.join("/") !== process.env.BROWSER1473_PAIR
          )
            continue;
          const [a, b] = direction,
            from = corpus[a].identity,
            to = corpus[b].identity;
          process.stderr.write(
            `corpus ${viewport.width} ${a}->${b} ${spread}\n`,
          );
          await page.goto(`${origin}/?${new URLSearchParams({ from, to })}`);
          await acknowledge(page);
          await page
            .getByTestId("swap-you-pay-amount")
            .waitFor({ timeout: 15000 })
            .catch(async (error) => {
              throw new Error(
                `Fixture entry unavailable: ${(await page.locator("body").innerText()).slice(0, 600)}; ${error.message}`,
              );
            });
          await page.getByRole("button", { name: "Settings" }).click();
          if (Number(spread) === 0.3) {
            await page
              .getByRole("textbox", {
                name: "Custom slippage protection (percent)",
              })
              .fill("30");
          } else {
            await page
              .getByTestId(`swap-slippage-preset-${Number(spread) * 100}`)
              .click();
          }
          const warm = economics(
            from,
            to,
            raw("0.1", corpus[a].decimals).toString(),
            spread,
          );
          let warmMs;
          currentStage = "warm";
          try {
            warmMs = await measure(
              page,
              "0.1",
              human(warm.output, corpus[b].decimals),
              warm.reason,
            );
          } catch (error) {
            failures.push({
              stage: "warm",
              viewport: viewport.width,
              pair: [a, b],
              max_spread: spread,
              amount: "0.1",
              expected_output: warm.output?.toString(),
              failure: safeError(error),
              ui: await page.evaluate(() => ({
                pay: document.querySelector(
                  '[aria-label="Select token you pay"]',
                )?.value,
                receive: document.querySelector(
                  '[aria-label="Select token you receive"]',
                )?.value,
                input: document.querySelector(
                  '[data-testid="swap-you-pay-amount"]',
                )?.value,
                output: document.querySelector(
                  '[data-testid="swap-you-receive"]',
                )?.value,
                busy: document
                  .querySelector('[data-testid="swap-you-receive"]')
                  ?.getAttribute("aria-busy"),
                error: document
                  .querySelector(
                    '[data-testid="swap-quote-error"], [data-testid="swap-quote-unavailable"], [data-testid="swap-quote-retry-error"]',
                  )
                  ?.textContent?.trim()
                  .slice(0, 256),
                measurement: {
                  start: window.__measure1473?.start,
                  end: window.__measure1473?.end,
                },
              })),
              trace: await measurementTrace(page),
              api_samples: requestSamples.slice(-8),
            });
            throw new Error(
              "Controlled fixture warm-up failed; inspect partial receipt",
            );
          }
          currentStage = "matrix";
          await page.evaluate(() => document.fonts.ready);
          await page.waitForLoadState("networkidle");
          const initializationMs = Date.now() - initializationStart;
          const amounts = [
            ...new Set([
              human(1n, corpus[a].decimals),
              human(2n, corpus[a].decimals),
              human(19n, corpus[a].decimals),
              ...matrix.human_amounts,
              ...boundaryAmounts(from, to, spread),
            ]),
          ];
          // Repeat amount transitions after a blocker; each direction is a separate receipt.
          for (const pass of ["first", "retest"])
            for (const amount of [...amounts, "10", "100", "1"]) {
              if (
                process.env.BROWSER1473_LIMIT &&
                scenarios >= Number(process.env.BROWSER1473_LIMIT)
              )
                break;
              const expected = economics(
                from,
                to,
                raw(amount, corpus[a].decimals).toString(),
                spread,
              );
              const startRequests = quoteRequests;
              try {
                const duration = await measure(
                  page,
                  amount,
                  expected.output
                    ? human(expected.output, corpus[b].decimals)
                    : "",
                  expected.reason,
                );
                const receipt = {
                  viewport: viewport.width,
                  pair: [a, b],
                  amount,
                  raw_input: raw(amount, corpus[a].decimals).toString(),
                  max_spread: spread,
                  initialization_ms: initializationMs,
                  cold_first_output_ms: warmMs,
                  transport_class:
                    quoteRequests === startRequests
                      ? "browser_cache_hit"
                      : "initialized_http",
                  pass,
                  outcome: expected.reason || "executable_quote",
                  expected_output: expected.output?.toString(),
                  useful_output_ms: expected.reason ? null : duration,
                  blocker_ms: expected.reason ? duration : null,
                  validated_execution_quote: !expected.reason,
                  request_attempts: quoteRequests - startRequests,
                  trace: await measurementTrace(page),
                };
                receipts.push(receipt);
                if (!expected.reason && duration > BROWSER_LATENCY_BUDGET_MS)
                  failures.push({
                    ...receipt,
                    failure: `initialized useful output exceeded ${BROWSER_LATENCY_BUDGET_MS}ms`,
                  });
              } catch (error) {
                failures.push({
                  viewport: viewport.width,
                  pair: [a, b],
                  amount,
                  max_spread: spread,
                  initialization_ms: initializationMs,
                  cold_first_output_ms: warmMs,
                  transport_class:
                    quoteRequests === startRequests
                      ? "browser_cache_hit"
                      : "initialized_http",
                  pass,
                  expected_reason: expected.reason || "executable_quote",
                  failure: error.message,
                  expected_output: expected.output?.toString(),
                  trace: await measurementTrace(page),
                  measurement: await page.evaluate(() => ({
                    start: window.__measure1473?.start,
                    end: window.__measure1473?.end,
                    busy: document
                      .querySelector('[data-testid="swap-you-receive"]')
                      ?.getAttribute("aria-busy"),
                  })),
                  observed_output: await page
                    .getByTestId("swap-you-receive")
                    .inputValue(),
                });
              }
              scenarios++;
            }
        }
    // Disconnected known assets are explicit negative controls; never quote
    // or sign a price just because both identities appear in discovery.
    if (!process.env.BROWSER1473_LIMIT) {
      const from = corpus.CUSTOM9.identity,
        to = corpus.CL8Y.identity;
      await page.goto(`${origin}/?${new URLSearchParams({ from, to })}`);
      await page.getByTestId("swap-you-pay-amount").waitFor();
      try {
        const duration = await measure(page, "1", "", "no_route");
        receipts.push({
          viewport: viewport.width,
          pair: ["CUSTOM9", "CL8Y"],
          amount: "1",
          outcome: "no_route",
          pass: "negative",
          useful_output_ms: null,
          blocker_ms: duration,
        });
      } catch (error) {
        failures.push({
          viewport: viewport.width,
          pair: ["CUSTOM9", "CL8Y"],
          failure: error.message,
        });
      }
    }
    if (externalRequests)
      failures.push({
        viewport: viewport.width,
        failure: "unexpected external requests",
        count: externalRequests,
      });
    await context.close();
  }
  if (providerRequests)
    failures.push({
      failure: "quote-triggered provider requests",
      count: providerRequests,
    });
  if (!receipts.length)
    failures.push({ failure: "No matrix scenarios completed" });
  currentStage = "complete";
  verificationFinished = true;
  writeReceipt(true);
  if (failures.length)
    throw new Error("Controlled browser matrix failed; inspect receipt");
}
function safeError(error) {
  return String(error.message || error)
    .split("\n")[0]
    .replace(/https?:\/\/\S+/g, "[fixture endpoint]")
    .slice(0, 256);
}
function writeReceipt(complete) {
  const receipt = {
    schema_version: 1,
    partial: !complete,
    stage: currentStage,
    frontend_artifact_sha256: fs.existsSync(path.join(buildDir, "index.html"))
      ? artifactDigest(buildDir)
      : null,
    matrix_sha256: createHash("sha256")
      .update(
        fs.readFileSync(path.join(root, "docs/qa/issue-1473/matrix.json")),
      )
      .digest("hex"),
    source_dirty: sourceDirtyAtBuild,
    recorded_at_utc: new Date().toISOString(),
    source_revision: sourceRevisionAtBuild,
    scope:
      "Controlled owned HTTP/browser integer constant-product fixture; production artifacts, actual settlement, nonzero wrapper fees and deployed acceptance are separate; native fixtures use zero-fee observed wrappers over genuine CW20 factory identities. Seven requested pairs plus arbitrary 9/12-decimal identities, both directions, first/retest. Disconnected wallet; no broadcast.",
    truncated:
      !!process.env.BROWSER1473_LIMIT ||
      !!process.env.BROWSER1473_PAIR ||
      !!process.env.BROWSER1473_SPREAD,
    browser_version: browser?.version(),
    browser_latency_budget_ms: BROWSER_LATENCY_BUDGET_MS,
    launch_flags: browserLaunchFlags,
    shared_memory_mode: sharedMemoryMode,
    measurement_clock: "original_input_event_through_confirmed_render_frame",
    provider_requests: providerRequests,
    quote_requests: quoteRequests,
    factory_pair_count: pairs.length,
    corpus_pair_count: pairLabels.length,
    useful_quote_latency: stats(
      receipts
        .filter((r) => r.outcome === "executable_quote")
        .map((r) => r.useful_output_ms),
    ),
    latency_by_class: Object.fromEntries(
      [1440, 390].flatMap((viewport) =>
        ["initialized_http", "browser_cache_hit"].map((kind) => [
          `${viewport}:${kind}`,
          stats(
            receipts
              .filter(
                (r) =>
                  r.viewport === viewport &&
                  r.transport_class === kind &&
                  r.outcome === "executable_quote",
              )
              .map((r) => r.useful_output_ms),
          ),
        ]),
      ),
    ),
    blocker_latency: stats(
      receipts
        .filter((r) => r.outcome !== "executable_quote")
        .map((r) => r.blocker_ms),
    ),
    receipts,
    failures,
  };
  const output =
    process.env.BROWSER1473_RECEIPT ||
    path.join(root, "docs/qa/issue-1473/browser-controlled.json");
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + "\n");
  process.stdout.write(
    JSON.stringify({
      output,
      scenarios: receipts.length,
      failures: failures.length,
      latency: receipt.useful_quote_latency,
    }) + "\n",
  );
  return receipt;
}

main()
  .catch((error) => {
    if (!verificationFinished) {
      if (!failures.some((failure) => failure.stage === currentStage))
        failures.push({ stage: currentStage, failure: safeError(error) });
      writeReceipt(false);
    }
    process.stderr.write(error.stack + "\n");
    process.exitCode = 1;
  })
  .finally(async () => {
    clearInterval(publisher);
    if (browser) await browser.close();
    server.close();
    provider.close();
    fs.rmSync(buildDir, { recursive: true, force: true });
  });
