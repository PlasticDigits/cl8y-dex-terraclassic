#!/usr/bin/env node
// Bounded read-only deployed browser matrix; no wallets, screenshots, broadcasts, or private bodies.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const {
  selectDirectionalGroups, amountCases, rawAmount, caseIdentity,
  expectedCaseIdentities, assertCaseCoverage,
} = require("./browser-quote-shards.cjs");
const { createRequire } = require("node:module");
try {
  const ROOT = path.resolve(__dirname, "../..");
  const requireFrontend = createRequire(
    path.join(ROOT, "frontend-dapp/package.json"),
  );
  const { chromium } = requireFrontend("@playwright/test");
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
  const SHA = process.env.EXPECTED_SOURCE_SHA;
  const OUTPUT =
    process.env.DEX1473_BROWSER_OUTPUT || "/tmp/dex1473-live-browser.json";
  const PROFILE = process.env.DEX1473_BROWSER_PROFILE || "full";
  const CYCLES = Number(process.env.DEX1473_BROWSER_CYCLES || "2");
  const QUOTE_LIMIT = 2200;
  const ELAPSED_LIMIT_MS = 30 * 60 * 1000;
  // Temporary verification limit requested for #1473; earlier 500 ms failures remain recorded.
  const BROWSER_LATENCY_BUDGET_MS = 750;
  const ATTESTATION = process.env.DEX1473_ATTESTATION_FILE;
  if (!ATTESTATION)
    throw new Error("Separate frontend/backend attestation file required");
  if (fs.statSync(ATTESTATION).size > 64 * 1024)
    throw new Error("Attestation body budget exceeded");
  const deployed = JSON.parse(fs.readFileSync(ATTESTATION, "utf8"));
  if (
    deployed.overall !== "PASS" ||
    deployed.expected_revision !== SHA ||
    deployed.frontend?.source_sha !== SHA ||
    deployed.backend?.git_sha !== SHA ||
    deployed.backend_deployment?.source_sha !== SHA ||
    deployed.frontend?.deployment_status !== "finished" ||
    deployed.backend_deployment?.deployment_status !== "finished"
  )
    throw new Error("Independent deployment provenance does not match target");
  if (
    !/^[0-9a-f]{40}$/.test(SHA || "") ||
    !["full", "census", "reproduction"].includes(PROFILE) ||
    ![1, 2].includes(CYCLES)
  )
    throw new Error("Explicit bounded revision/profile/cycles required");
  const attestedArtifact = deployed.frontend?.artifact;
  if (
    !/^[0-9a-f]{64}$/.test(attestedArtifact?.html_sha256 || "") ||
    !Array.isArray(attestedArtifact?.root_scripts) ||
    !attestedArtifact.root_scripts.length ||
    attestedArtifact.root_scripts.length > 8 ||
    attestedArtifact.root_scripts.some(
      (row) =>
        !row ||
        !/^[-A-Za-z0-9_.]+\.js$/.test(row.asset) ||
        !/^[0-9a-f]{64}$/.test(row.sha256 || ""),
    )
  )
    throw new Error(
      "Complete independently hashed root artifact attestation required",
    );
  const MATRIX_ISSUE = process.env.DEX1473_BROWSER_MATRIX || "1473";
  if (!["1473", "1479"].includes(MATRIX_ISSUE))
    throw new Error("Explicit bounded matrix identity required");
  const matrixBytes = fs.readFileSync(
    path.join(ROOT, `docs/qa/issue-${MATRIX_ISSUE}/matrix.json`),
  );
  const matrix = JSON.parse(matrixBytes);
  const pairCorpus =
    PROFILE === "reproduction" ? [matrix.pairs[0]] : matrix.pairs;
  const humanAmounts =
    PROFILE === "reproduction"
      ? ["1", "10", "100", "1000"]
      : PROFILE === "census"
        ? ["0.000001", "0.01", "1", "100", "10000"]
        : matrix.human_amounts;
  const matrixIdentity = crypto.createHash("sha256").update(matrixBytes).digest("hex");
  const selection = selectDirectionalGroups(pairCorpus, process.env);
  const casePlan = {
    matrixIdentity, profile: PROFILE, cycles: CYCLES,
    assets: matrix.assets, spreads: matrix.max_spreads, humanAmounts,
  };
  const corpusCaseIds = expectedCaseIdentities({ ...casePlan, groups: selection.corpus });
  const selectedCaseIds = expectedCaseIdentities({ ...casePlan, groups: selection.selected });
  const FRONTEND = "https://dex.cl8y.com";
  const BACKEND = "https://indexer.dex.cl8y.com";
  let ownedBrowser,
    quoteRequests = 0,
    networkQuoteRequests = 0,
    preventedWrites = 0,
    blockedWebSockets = 0,
    elapsedBudgetExpired = false;
  const runStartedAt = Date.now();
  const receipt = {
    issue: Number(MATRIX_ISSUE),
    launch_flags: browserLaunchFlags,
    shared_memory_mode: sharedMemoryMode,
    measurement_clock: "original_input_event_through_confirmed_render_frame",
    elapsed_time_limit_ms: ELAPSED_LIMIT_MS,
    browser_latency_budget_ms: BROWSER_LATENCY_BUDGET_MS,
    viewport_setup: "settings_closed_and_both_fields_intersect_before_input",
    expected_revision: SHA,
    matrix_sha256: matrixIdentity,
    shard: selection.shard,
    corpus_expected_sample_count: corpusCaseIds.length,
    selected_expected_sample_count: selectedCaseIds.length,
    expected_case_ids: selectedCaseIds,
    profile: PROFILE,
    cycles: CYCLES,
    utc_start: new Date().toISOString(),
    read_only: true,
    screenshots: false,
    signing: false,
    completed: false,
    expected_sample_count: selectedCaseIds.length,
    samples: [],
    artifact_hashes: [],
    backend_revisions: [],
    prohibited_write_attempts: 0,
  };
  const raw = rawAmount;
  function stats(rows) {
    const values = rows
      .filter((row) => row.useful)
      .map((row) => row.input_to_dom_ms)
      .sort((a, b) => a - b);
    const at = (p) =>
      values[Math.max(0, Math.ceil(values.length * p) - 1)] ?? null;
    return {
      count: values.length,
      p50_ms: at(0.5),
      p95_ms: at(0.95),
      p99_ms: at(0.99),
      max_ms: at(1),
    };
  }
  function assertElapsedBudget() {
    if (elapsedBudgetExpired || Date.now() - runStartedAt >= ELAPSED_LIMIT_MS)
      throw new Error("Browser elapsed-time budget reached");
  }
  async function boundedFetch(url, limit) {
    assertElapsedBudget();
    const response = await fetch(url, {
      signal: AbortSignal.timeout(
        Math.max(
          1,
          Math.min(10000, ELAPSED_LIMIT_MS - (Date.now() - runStartedAt)),
        ),
      ),
      redirect: "error",
      headers: { "Cache-Control": "no-cache" },
    });
    if (!response.ok) throw new Error("Read prerequisite unavailable");
    const reader = response.body.getReader();
    let size = 0;
    const chunks = [];
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > limit) {
        await reader.cancel();
        throw new Error("Read body budget exceeded");
      }
      chunks.push(Buffer.from(value));
    }
    return Buffer.concat(chunks);
  }
  async function attestBackend() {
    const body = JSON.parse(await boundedFetch(BACKEND + "/health", 4096));
    if (
      typeof body.git_sha !== "string" ||
      !/^[0-9a-f]{40}$/.test(body.git_sha)
    )
      throw new Error("Backend public revision unavailable");
    receipt.backend_revisions.push(body.git_sha);
    if (body.git_sha !== SHA)
      throw new Error("Backend revision mismatch; no deployed acceptance");
  }
  async function attestArtifacts() {
    const html = await boundedFetch(FRONTEND + "/", 128 * 1024);
    const paths = [
      ...html.toString().matchAll(/<script\b[^>]*\bsrc=["']([^"']+)["']/g),
    ]
      .map((match) => match[1])
      .filter((asset) => asset.startsWith("/assets/") && asset.endsWith(".js"));
    if (
      !paths.length ||
      paths.length > 8 ||
      paths.some((asset) => !/^\/assets\/[A-Za-z0-9_.-]+\.js$/.test(asset))
    )
      throw new Error("Frontend artifact unavailable");
    const result = [];
    for (const assetPath of paths) {
      const bytes = await boundedFetch(FRONTEND + assetPath, 8 * 1024 * 1024);
      result.push({
        asset: path.basename(assetPath),
        sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
      });
    }
    return {
      html_sha256: crypto.createHash("sha256").update(html).digest("hex"),
      root_scripts: result,
    };
  }
  function save() {
    receipt.utc_end = new Date().toISOString();
    receipt.quote_request_count = quoteRequests;
    receipt.quote_request_limit = QUOTE_LIMIT;
    receipt.network_quote_request_count = networkQuoteRequests;
    receipt.prohibited_write_attempts = preventedWrites;
    receipt.blocked_websocket_count = blockedWebSockets;
    receipt.useful_latency = stats(receipt.samples);
    receipt.latency_by_cache = Object.fromEntries(
      ["hit", "miss", "unobserved"].map((kind) => [
        kind,
        stats(receipt.samples.filter((row) => row.server_cache_class === kind)),
      ]),
    );
    receipt.latency_by_viewport = Object.fromEntries(
      ["desktop", "phone"].map((view) => [
        view,
        stats(receipt.samples.filter((row) => row.viewport === view)),
      ]),
    );
    receipt.latency_by_transport = Object.fromEntries(
      ["new", "reused", "browser_cache_hit", "unobserved"].map((kind) => [
        kind,
        stats(receipt.samples.filter((row) => row.transport === kind)),
      ]),
    );
    receipt.overall =
      receipt.failure_class ||
      elapsedBudgetExpired ||
      receipt.samples.some((row) => row.failed) ||
      preventedWrites
        ? "FAIL"
        : receipt.completed
          ? "PASS WITH SKIPS"
          : "RUNNING";
    receipt.skips = [
      "Physical device/cellular tests",
      "Owned chain settlement",
      "Historical pinned production economics",
    ];
    fs.writeFileSync(OUTPUT, JSON.stringify(receipt, null, 2) + "\n", {
      mode: 0o600,
    });
  }
  // Configure the visible ticket before installing the original-input clock.
  // Expanded settings can put the receive field below a narrow viewport.
  async function prepareMeasurementViewport(page) {
    const settings = page.locator("#swap-slippage-settings");
    if (await settings.isVisible())
      await page.getByRole("button", { name: "Settings", exact: true }).click();
    await settings.waitFor({ state: "hidden", timeout: 5000 });
    await page.getByTestId("swap-you-pay-amount").evaluate((pay) => {
      const doc = pay.ownerDocument,
        receive = doc.querySelector('[data-testid="swap-you-receive"]'),
        view = doc.defaultView;
      if (!receive || !view) throw new Error("Quote ticket unavailable");
      const a = pay.getBoundingClientRect(),
        b = receive.getBoundingClientRect();
      view.scrollBy({
        top:
          (Math.min(a.top, b.top) + Math.max(a.bottom, b.bottom)) / 2 -
          view.innerHeight / 2,
        behavior: "instant",
      });
    });
    await page.waitForFunction(
      () => {
        return ["swap-you-pay-amount", "swap-you-receive"].every((id) => {
          const node = document.querySelector(`[data-testid="${id}"]`),
            rect = node?.getBoundingClientRect(),
            style = node ? getComputedStyle(node) : null;
          return (
            rect?.width > 0 &&
            rect?.height > 0 &&
            rect.right > 0 &&
            rect.bottom > 0 &&
            rect.left < innerWidth &&
            rect.top < innerHeight &&
            style?.display !== "none" &&
            style?.visibility !== "hidden" &&
            Number(style?.opacity) > 0
          );
        });
      },
      undefined,
      { timeout: 5000 },
    );
  }
  async function acknowledge(page) {
    const risk = page.getByRole("checkbox", {
      name: /I understand these risks/,
    });
    if (await risk.isVisible().catch(() => false)) {
      await risk.check();
      await page.getByRole("button", { name: "Continue to the app" }).click();
    }
  }
  async function main() {
    await attestBackend();
    const initialArtifacts = await attestArtifacts();
    const attestedScripts = attestedArtifact.root_scripts.map(
      ({ asset, sha256 }) => ({ asset, sha256 }),
    );
    if (
      initialArtifacts.html_sha256 !== attestedArtifact.html_sha256 ||
      JSON.stringify(initialArtifacts.root_scripts) !==
        JSON.stringify(attestedScripts)
    )
      throw new Error(
        "Frontend served bytes differ from independent attestation",
      );
    receipt.artifact_hashes.push({ phase: "before", assets: initialArtifacts });
    ownedBrowser = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: browserLaunchFlags,
    });
    receipt.default_entry_checks = [];
    for (let cycle = 0; cycle < CYCLES; cycle++) {
      if (cycle) await new Promise((resolve) => setTimeout(resolve, 181000));
      await attestBackend();
      for (const [view, viewport] of [
        ["desktop", { width: 1440, height: 1000 }],
        ["phone", { width: 390, height: 844 }],
      ]) {
        const context = await ownedBrowser.newContext({
          viewport,
          serviceWorkers: "block",
        });
        await context.routeWebSocket("**/*", (socket) => {
          blockedWebSockets++;
          receipt.failure_class = "UnsupportedWebSocket";
          socket.close();
        });
        await context.exposeBinding("__record1473ProhibitedWrite", () => {
          preventedWrites++;
        });
        await context.exposeBinding("__reserve1473QuoteRead", () => {
          assertElapsedBudget();
          if (quoteRequests >= QUOTE_LIMIT) {
            receipt.failure_class = "QuoteReadBudget";
            return false;
          }
          quoteRequests++;
          return true;
        });
        await context.addInitScript(() => {
          const S = 10n ** 18n,
            M = (1n << 128n) - 1n;
          const integer = (value) =>
            typeof value === "string" &&
            /^(0|[1-9]\d{0,38})$/.test(value) &&
            BigInt(value) <= M
              ? BigInt(value)
              : null;
          const decimal = (value) =>
            typeof value === "string" && /^(0|0\.\d{1,18}|1)$/.test(value)
              ? BigInt(
                  value === "1"
                    ? "1000000000000000000"
                    : (value.split(".")[1] || "").padEnd(18, "0"),
                )
              : null;
          const reasons = {
            protection: 422,
            price_impact_too_high: 422,
            amount_too_small: 422,
            amount_too_large: 422,
            insufficient_liquidity: 422,
            unsupported_semantics: 422,
            reference_policy_excluded: 422,
            trading_blocked: 422,
            no_route: 422,
            missing_observations: 503,
            stale_observations: 503,
            accounting_unavailable: 503,
            invalid_request: 400,
          };
          window.__quotes1473 = [];
          window.__writes1473 = 0;
          function validate(body, params, now) {
            const m = body.execution_metadata,
              ops = body.router_operations,
              net = integer(body.estimated_amount_out_net),
              fair = integer(body.spot_amount_out);
            const u = decimal(params.max_spread),
              minimum = integer(m?.minimum_receive);
            if (
              !m ||
              (Object.hasOwn(m, "execution_supported") &&
                typeof m.execution_supported !== "boolean") ||
              (Object.hasOwn(body, "execution_supported") &&
                (typeof body.execution_supported !== "boolean" ||
                  body.execution_supported !== (m.execution_supported ?? true)))
            )
              return false;
            if (
              !m ||
              m.protection_policy !== "quote_relative_v1" ||
              decimal(m.max_spread) !== u ||
              u == null ||
              u <= 0n ||
              u > S / 2n ||
              net == null ||
              net <= 0n ||
              fair == null ||
              fair <= 0n ||
              minimum == null ||
              minimum < (net * (S - u) + S - 1n) / S ||
              minimum > net ||
              body.token_in !== params.token_in ||
              body.token_out !== params.token_out ||
              m.input_amount !== params.amount_in ||
              m.execution_supported === false ||
              body.execution_blocker != null ||
              m.trader != null ||
              m.sender != null ||
              !Number.isSafeInteger(m.observed_at_ms) ||
              m.observed_at_ms < 0 ||
              m.observed_at_ms > now ||
              !Number.isSafeInteger(m.expires_at_ms) ||
              m.expires_at_ms <= now ||
              m.expires_at_ms <= m.observed_at_ms ||
              m.expires_at_ms - m.observed_at_ms > 20000 ||
              typeof m.generation !== "string" ||
              !/^[a-zA-Z0-9:_-]{1,256}$/.test(m.generation) ||
              !Array.isArray(m.hops) ||
              !Array.isArray(ops) ||
              !m.hops.length ||
              m.hops.length > 4 ||
              m.hops.length !== ops.length
            )
              return false;
            if (
              (fair > net ? fair - net : net - fair) * 100n >=
              (fair > net ? fair : net) * 99n
            )
              return false;
            let ceiling = 0n;
            for (let index = 0; index < m.hops.length; index++) {
              const h = m.hops[index],
                t = ops[index]?.terra_swap;
              const pool = integer(h.pool_return_amount),
                fee = integer(h.commission_amount),
                book = integer(h.book_return_net),
                poolInput = integer(h.pool_input_actual),
                consumed = integer(h.book_offer_consumed),
                spread = integer(h.spread_amount),
                output = integer(h.return_amount),
                floor = integer(h.min_return);
              if (
                [
                  pool,
                  fee,
                  book,
                  poolInput,
                  consumed,
                  spread,
                  output,
                  floor,
                ].some((value) => value == null) ||
                output <= 0n ||
                floor < (output * (S - u) + S - 1n) / S ||
                floor > output ||
                t?.min_return !== h.min_return
              )
                return false;
              const D = pool + fee + book,
                expectedBook =
                  poolInput > 0n && consumed > 0n
                    ? (pool * consumed) / poolInput
                    : 0n,
                N =
                  (spread < pool + fee ? spread : pool + fee) +
                  (expectedBook > book ? expectedBook - book : 0n);
              if (D <= 0n || D > M || N > D) return false;
              const numerator = (N * S + D * u) * S,
                denominator = D * (S - u),
                derived = (numerator + denominator - 1n) / denominator,
                capped = derived > S ? S : derived;
              if (capped > ceiling) ceiling = capped;
            }
            return decimal(m.execution_market_spread) === ceiling;
          }
          const original = window.fetch.bind(window);
          window.fetch = async (...args) => {
            const input = args[0],
              url = new URL(
                typeof input === "string" ? input : input?.url || String(input),
                location.href,
              ),
              method = String(
                args[1]?.method || input?.method || "GET",
              ).toUpperCase();
            if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
              window.__writes1473++;
              await window.__record1473ProhibitedWrite();
              throw new Error("Non-read request prohibited");
            }
            const quote =
                method === "GET" &&
                ["/api/v1/route/solve", "/api/v1/route/solve/best"].includes(
                  url.pathname,
                ),
              started = performance.now();
            if (quote && !(await window.__reserve1473QuoteRead()))
              throw new Error("Quote read budget exhausted");
            const params = quote ? Object.fromEntries(url.searchParams) : null;
            const row = quote ? { started, status: null, params } : null;
            if (row) {
              window.__quotes1473.push(row);
              if (window.__quotes1473.length > 2200)
                window.__quotes1473.shift();
            }
            let response;
            try {
              response = await original(...args);
            } catch (error) {
              if (row) {
                row.transport_failed = true;
                row.ended = performance.now();
              }
              throw error;
            }
            if (quote) {
              row.status = response.status;
              row.headers_received = performance.now();
              void (async () => {
                try {
                  const reader = response.clone().body.getReader(),
                    limit = response.status === 200 ? 2 * 1024 * 1024 : 4096;
                  let count = 0;
                  const chunks = [];
                  while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    count += value.length;
                    if (count > limit) {
                      await reader.cancel();
                      throw new Error("Body budget");
                    }
                    chunks.push(value);
                  }
                  const bytes = new Uint8Array(count);
                  let offset = 0;
                  for (const chunk of chunks) {
                    bytes.set(chunk, offset);
                    offset += chunk.length;
                  }
                  const body = JSON.parse(new TextDecoder().decode(bytes));
                  row.reason =
                    Object.hasOwn(reasons, body.error) &&
                    reasons[body.error] === response.status
                      ? body.error
                      : undefined;
                  row.classified =
                    response.status === 200
                      ? validate(body, params, Date.now())
                      : row.reason != null;
                  row.useful = response.status === 200 && row.classified;
                  row.output = row.useful
                    ? body.estimated_amount_out_net
                    : undefined;
                  row.minimum = row.useful
                    ? body.execution_metadata.minimum_receive
                    : undefined;
                  row.observed = row.useful
                    ? body.execution_metadata.observed_at_ms
                    : undefined;
                  row.expires = row.useful
                    ? body.execution_metadata.expires_at_ms
                    : undefined;
                  const metadata = body.execution_metadata;
                  row.plan_policy = ["pool_only", "declared_hybrid_hop0", "declared_hybrid_manual", "wrapper_only"].includes(metadata?.policy) ? metadata.policy : null;
                  row.hop_count = Array.isArray(metadata?.hops) && metadata.hops.length <= 4 ? metadata.hops.length : null;
                  row.operation_count = Array.isArray(body.router_operations) && body.router_operations.length <= 4 ? body.router_operations.length : null;
                  row.wrapper_present = metadata?.wrapper != null;
                  row.generation_valid =
                    typeof body.execution_metadata?.generation === "string";
                  row.generation = body.execution_metadata?.generation;
                } catch {
                  row.classified = false;
                }
                row.ended = performance.now();
              })().catch(() => {});
            }
            return response;
          };
        });
        await context.route("**/*", async (route) => {
          const request = route.request(),
            method = request.method();
          if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
            preventedWrites++;
            await route.abort("blockedbyclient");
            return;
          }
          const url = new URL(request.url());
          if (
            elapsedBudgetExpired ||
            Date.now() - runStartedAt >= ELAPSED_LIMIT_MS
          ) {
            elapsedBudgetExpired = true;
            await route.abort("blockedbyclient");
            return;
          }
          if (
            ["/api/v1/route/solve", "/api/v1/route/solve/best"].includes(
              url.pathname,
            ) &&
            method === "GET"
          ) {
            if (networkQuoteRequests >= QUOTE_LIMIT) {
              receipt.failure_class = "QuoteReadBudget";
              await route.abort("blockedbyclient");
              return;
            }
            networkQuoteRequests++;
          }
          await route.continue();
        });
        const page = await context.newPage(),
          cdp = await context.newCDPSession(page);
        await cdp.send("Network.enable");
        let connections = [];
        cdp.on("Network.responseReceived", (event) => {
          const url = new URL(event.response.url);
          if (
            ["/api/v1/route/solve", "/api/v1/route/solve/best"].includes(
              url.pathname,
            )
          ) {
            connections.push({
              amount: url.searchParams.get("amount_in"),
              input: url.searchParams.get("token_in"),
              output: url.searchParams.get("token_out"),
              spread: url.searchParams.get("max_spread"),
              reused: event.response.connectionReused,
              protocol: ["h2", "h3", "http/1.1"].includes(
                event.response.protocol,
              )
                ? event.response.protocol
                : "unobserved",
              at: Date.now(),
            });
            if (connections.length > 2200) connections.shift();
          }
        });
        for (const [entry, label] of [
          ["/", "swap"],
          ["/trade", "trade"],
        ]) {
          await page.goto(FRONTEND + entry, {
            waitUntil: "domcontentloaded",
            timeout: 30000,
          });
          await acknowledge(page);
          await page
            .getByTestId("swap-you-pay-amount")
            .waitFor({ timeout: 20000 });
          await page.waitForFunction(
            () =>
              document.querySelector('[aria-label="Select token you pay"]')
                ?.value === "USTC" &&
              document.querySelector('[aria-label="Select token you receive"]')
                ?.value === "CL8Y",
            undefined,
            { timeout: 20000 },
          );
          const payLabel = await page
              .getByRole("combobox", { name: "Select token you pay" })
              .inputValue(),
            receiveLabel = await page
              .getByRole("combobox", { name: "Select token you receive" })
              .inputValue();
          const selected = {
            viewport: view,
            entry: label,
            native_ustc: payLabel === "USTC",
            configured_cl8y: receiveLabel === "CL8Y",
          };
          receipt.default_entry_checks.push(selected);
          if (!selected.native_ustc || !selected.configured_cl8y)
            throw new Error("Default entry policy mismatch");
        }
        for (const group of selection.selected)
          for (const spread of matrix.max_spreads) {
              const {from, to} = group,
                a = matrix.assets[from],
                b = matrix.assets[to];
              await page.goto(
                FRONTEND +
                  "/?" +
                  new URLSearchParams({ from: a.identity, to: b.identity }),
                { waitUntil: "domcontentloaded", timeout: 30000 },
              );
              await acknowledge(page);
              const pay = page.getByTestId("swap-you-pay-amount");
              await pay.waitFor({ timeout: 20000 });
              await page.waitForFunction(
                ({ from, to }) =>
                  document.querySelector('[aria-label="Select token you pay"]')
                    ?.value === from &&
                  document.querySelector(
                    '[aria-label="Select token you receive"]',
                  )?.value === to,
                { from, to },
                { timeout: 20000 },
              );
              await page
                .getByRole("button", { name: "Settings", exact: true })
                .click();
              if (spread === "0.30")
                await page
                  .getByLabel("Custom slippage protection (percent)", {
                    exact: true,
                  })
                  .fill("30");
              else
                await page
                  .getByTestId("swap-slippage-preset-" + Number(spread) * 100)
                  .click();
              await prepareMeasurementViewport(page);
              await pay.fill("0.1");
              await page.waitForTimeout(1000);
              const amounts = amountCases(PROFILE, humanAmounts, a.decimals);
              for (const amountCase of amounts) {
                const amount = amountCase.human;
                assertElapsedBudget();
                if (quoteRequests >= QUOTE_LIMIT)
                  throw new Error("Quote request limit reached");
                await pay.fill("0");
                await page.waitForTimeout(75);
                const inputRaw = raw(amount, a.decimals);
                let cacheObservation = null;
                try {
                  const progress = JSON.parse(
                    await boundedFetch(
                      BACKEND +
                        "/api/v1/route/solve/progress?" +
                        new URLSearchParams({
                          token_in: a.identity,
                          token_out: b.identity,
                          amount_in: inputRaw,
                          max_spread: spread,
                          max_maker_fills: "8",
                          protection_policy: "quote_relative_v1",
                          ...(!a.identity.startsWith("terra1") ||
                          !b.identity.startsWith("terra1")
                            ? { pool_only: "true" }
                            : {}),
                        }),
                      8192,
                    ),
                  );
                  if (
                    typeof progress.cache_hit === "boolean" &&
                    typeof progress.quote_generation === "string"
                  )
                    cacheObservation = {
                      hit: progress.cache_hit,
                      generation: progress.quote_generation,
                    };
                } catch {}
                await prepareMeasurementViewport(page);
                const nodeStart = Date.now();
                await page.evaluate(
                  (expected) => {
                    const measurement = (window.__measurement1473 = {
                        started: null,
                        done: null,
                        kind: null,
                        timeline: [],
                        readiness: { first_ready_at_ms: null, first_frame_at_ms: null, paint_frame_at_ms: null, receive_visible: null, surface_visible: null, kind: null },
                      }),
                      selector = '[data-testid="swap-you-pay-amount"]';
                    const waitingSelector = '[data-testid="swap-price-data-loading"]';
                    const errorSelector =
                      '[data-testid="swap-quote-error"],[data-testid="swap-quote-unavailable"],[data-testid="swap-quote-retry-error"],[data-testid="swap-wrapper-quote-error"]';
                    const reasonCopy = {
                      protection: "selected price protection",
                      price_impact_too_high: "Fair-reference deviation is too high",
                      amount_too_small:
                        "too small to receive a positive output",
                      amount_too_large: "exceeds the supported trade limits",
                      insufficient_liquidity: "insufficient liquidity",
                      unsupported_semantics: "not supported by current pricing",
                      reference_policy_excluded: "Reliable reference pricing is unavailable",
                      trading_blocked: "Trading is restricted",
                      no_route: "No supported route connects",
                      missing_observations:
                        "Pricing is temporarily unavailable",
                      stale_observations: "Pricing has expired",
                      accounting_unavailable:
                        "Pricing could not be calculated safely",
                      invalid_request: "Check the trade amount and settings",
                    };
                    const matches = (row) =>
                      row.params.token_in === expected.input &&
                      row.params.token_out === expected.output &&
                      row.params.amount_in === expected.amount &&
                      Number(row.params.max_spread) ===
                        Number(expected.spread) &&
                      row.params.protection_policy === "quote_relative_v1";
                    const errorText = () =>
                      Array.from(document.querySelectorAll(errorSelector))
                        .map((node) => node.textContent || "")
                        .join(" ");
                    // Capture bounded state changes without reading full-document text or layout.
                    const parseAmount = (value, decimals) => {
                      if (typeof value !== "string" || value.length > 96 || !/^\d+(\.\d+)?$/.test(value) || !Number.isInteger(decimals) || decimals < 0 || decimals > 38) return null;
                      const [whole, fraction = ""] = value.split(".");
                      if (fraction.length > decimals) return null;
                      return (BigInt(whole) * 10n ** BigInt(decimals) + BigInt(fraction.padEnd(decimals, "0") || "0")).toString();
                    };
                    const timelineState = () => {
                      const rows = window.__quotes1473.filter(matches);
                      const current = rows.filter((row) => row.started >= measurement.started);
                      const prior = rows.filter((row) => row.started < measurement.started).at(-1);
                      const latest = current.at(-1) || prior;
                      const output = document.querySelector('[data-testid="swap-you-receive"]');
                      const value = output?.value?.replaceAll(",", "") || "";
                      return {
                        current_input_matches: parseAmount(document.querySelector(selector)?.value, expected.offerDecimals) === expected.amount,
                        receive_present: value !== "",
                        receive_busy: output?.getAttribute("aria-busy") === "true",
                        receive_matches_quote: !!latest?.useful && parseAmount(value, expected.decimals) === latest.output,
                        quote_request_count: current.length,
                        response_received: latest?.headers_received != null || latest?.status != null,
                        validation_complete: latest?.ended != null,
                        quote_validated: latest?.classified === true,
                        matched_quote_expired: latest?.useful === true && latest.expires <= Date.now(),
                        expired_prior_quote: prior?.useful === true && prior.expires <= Date.now(),
                        waiting_surface_present: document.querySelector(waitingSelector) != null,
                      };
                    };
                    const captureState = () => {
                      if (measurement.started == null) return;
                      const state = timelineState(), signature = JSON.stringify(state);
                      if (signature !== measurement.lastState && measurement.timeline.length < 24) {
                        measurement.lastState = signature;
                        measurement.timeline.push({ at_ms: performance.now() - measurement.started, ...state });
                      }
                    };
                    const diagnostics = () => {
                      const current = window.__quotes1473.filter((row) => matches(row) && row.started >= measurement.started);
                      const prior = window.__quotes1473.filter((row) => matches(row) && row.started < measurement.started).slice(-4);
                      const delta = (value) => Number.isFinite(value) ? value - measurement.started : null;
                      const describe = (row) => ({
                        request_at_ms: delta(row.started),
                        headers_at_ms: delta(row.headers_received),
                        validated_at_ms: delta(row.ended),
                        status: Number.isInteger(row.status) && row.status >= 100 && row.status <= 599 ? row.status : null,
                        reason: Object.hasOwn(reasonCopy, row.reason) ? row.reason : null,
                        classified: row.classified === true,
                        transport_failed: row.transport_failed === true,
                        expiry_at_ms: Number.isSafeInteger(row.expires) ? row.expires - Date.now() + performance.now() - measurement.started : null,
                        plan_policy: ["pool_only", "declared_hybrid_hop0", "declared_hybrid_manual", "wrapper_only"].includes(row.plan_policy) ? row.plan_policy : null,
                        hop_count: Number.isInteger(row.hop_count) && row.hop_count >= 0 && row.hop_count <= 4 ? row.hop_count : null,
                        operation_count: Number.isInteger(row.operation_count) && row.operation_count >= 0 && row.operation_count <= 4 ? row.operation_count : null,
                        wrapper_present: row.wrapper_present === true,
                      });
                      return {
                        no_current_quote_request: current.length === 0,
                        current_request_count: current.length,
                        current_records_truncated: current.length > 12,
                        current_records: (current.length > 12 ? [...current.slice(0, 6), ...current.slice(-6)] : current).map(describe),
                        previous_records: prior.map(describe),
                        final_state: timelineState(),
                        readiness: measurement.readiness,
                        timeline: measurement.timeline,
                      };
                    };
                    const start = (event) => {
                      if (
                        event.target.matches?.(selector) &&
                        measurement.started == null
                      )
                        { measurement.started = performance.now(); captureState(); }
                    };
                    document.addEventListener("input", start, true);
                    const ready = () => {
                      if (measurement.started == null) return null;
                      const records = window.__quotes1473.filter(
                        (row) =>
                          row.started >= measurement.started && matches(row),
                      );
                      const latest =
                        records.at(-1) ||
                        window.__quotes1473
                          .filter(
                            (row) =>
                              matches(row) &&
                              row.useful &&
                              row.expires > Date.now(),
                          )
                          .at(-1);
                      if (!latest || latest.ended == null) return null;
                      const output = document.querySelector(
                        '[data-testid="swap-you-receive"]',
                      );
                      if (!output) return null;
                      const value = output.value?.replaceAll(",", "") || "";
                      if (
                        latest.useful &&
                        latest.expires > Date.now() &&
                        value &&
                        output.getAttribute("aria-busy") !== "true"
                      ) {
                        const [whole, fraction = ""] = value.split(".");
                        if (
                          !/^\d+(\.\d+)?$/.test(value) ||
                          fraction.length > expected.decimals
                        )
                          return null;
                        const displayed =
                          BigInt(whole) * 10n ** BigInt(expected.decimals) +
                          BigInt(
                            fraction.padEnd(expected.decimals, "0") || "0",
                          );
                        if (displayed === BigInt(latest.output))
                          return { kind: "useful", latest, output };
                      } else if (latest.classified && !latest.useful && value === "") {
                        const waiting = document.querySelector(waitingSelector);
                        if (["missing_observations", "stale_observations"].includes(latest.reason) && waiting?.querySelector('progress:not([value]),[role="progressbar"]:not([aria-valuenow])') && (waiting.textContent || "").includes("Loading price"))
                          return { kind: "waiting", latest, output, surface: waiting };
                        if (Object.hasOwn(reasonCopy, latest.reason) && errorText().includes(reasonCopy[latest.reason]))
                          return { kind: "blocked", latest, output };
                      }
                      return null;
                    };
                    const visible = (node) => {
                      const rect = node?.getBoundingClientRect(),
                        style = node ? getComputedStyle(node) : null;
                      return (
                        rect?.width > 0 &&
                        rect?.height > 0 &&
                        rect.right > 0 &&
                        rect.bottom > 0 &&
                        rect.left < innerWidth &&
                        rect.top < innerHeight &&
                        style?.display !== "none" &&
                        style?.visibility !== "hidden" &&
                        Number(style?.opacity) > 0
                      );
                    };
                    const check = () => {
                      if (
                        measurement.started == null ||
                        measurement.done != null ||
                        measurement.readyFrame != null ||
                        !ready()
                      )
                        return;
                      // The original event clock includes all retries and two render-frame opportunities.
                      if (measurement.readiness.first_ready_at_ms == null) measurement.readiness.first_ready_at_ms = performance.now() - measurement.started;
                      measurement.readyFrame = requestAnimationFrame(() => {
                        if (measurement.readiness.first_frame_at_ms == null) measurement.readiness.first_frame_at_ms = performance.now() - measurement.started;
                        measurement.paintFrame = requestAnimationFrame(() => {
                          const current = ready();
                          const surface =
                            current?.kind === "useful"
                              ? current.output
                              : current?.kind === "waiting"
                                ? current.surface
                              : Array.from(
                                  document.querySelectorAll(errorSelector),
                                ).find(
                                  (node) =>
                                    reasonCopy[current?.latest.reason] &&
                                    (node.textContent || "").includes(
                                      reasonCopy[current.latest.reason],
                                    ),
                                );
                          measurement.readiness.paint_frame_at_ms = performance.now() - measurement.started;
                          measurement.readiness.receive_visible = current ? Boolean(visible(current.output)) : null;
                          measurement.readiness.surface_visible = current ? Boolean(visible(surface)) : null;
                          measurement.readiness.kind = current?.kind ?? null;
                          if (
                            current &&
                            (current.kind !== "useful" || measurement.readiness.receive_visible) &&
                            measurement.readiness.surface_visible
                          ) {
                            measurement.kind = current.kind;
                            measurement.latest = current.latest;
                            measurement.done = performance.now();
                          } else measurement.readyFrame = null;
                        });
                      });
                    };
                    measurement.frame = requestAnimationFrame(check);
                    measurement.timer = setInterval(() => { captureState(); check(); }, 5);
                    measurement.read = () => {
                      clearInterval(measurement.timer);
                      cancelAnimationFrame(measurement.frame);
                      cancelAnimationFrame(measurement.readyFrame);
                      cancelAnimationFrame(measurement.paintFrame);
                      document.removeEventListener("input", start, true);
                      const records = window.__quotes1473.filter(
                        (row) =>
                          row.started >= measurement.started &&
                          row.params.token_in === expected.input &&
                          row.params.token_out === expected.output &&
                          row.params.amount_in === expected.amount,
                      );
                      return {
                        diagnostics: diagnostics(),
                        useful: measurement.kind === "useful",
                        classified: measurement.kind != null,
                        input_to_dom_ms:
                          measurement.done == null
                            ? null
                            : measurement.done - measurement.started,
                        attempts: records.length,
                        statuses: records.map((row) => row.status),
                        outcome:
                          measurement.latest?.reason ||
                          (measurement.kind === "useful"
                            ? "executable"
                            : "unavailable"),
                        server_cache_class:
                          expected.cacheObservation &&
                          measurement.latest?.generation ===
                            expected.cacheObservation.generation
                            ? expected.cacheObservation.hit
                              ? "hit"
                              : "miss"
                            : "unobserved",
                        net_output: measurement.latest?.output,
                        minimum_receive: measurement.latest?.minimum,
                        api_ms:
                          measurement.latest?.ended -
                          measurement.latest?.started,
                        cached_original_quote:
                          records.length === 0 && measurement.kind === "useful",
                        original_observed_at_ms: measurement.latest?.observed,
                        expires_at_ms: measurement.latest?.expires,
                        policy_opt_in: records.every(
                          (row) =>
                            row.params.protection_policy ===
                            "quote_relative_v1",
                        ),
                        secondary_estimate_present:
                          document.querySelector(
                            '[data-testid="swap-indicative-estimate"]',
                          ) != null,
                        generic_validation_copy:
                          /route validation (rejected|failed)/i.test(
                            errorText(),
                          ),
                      };
                    };
                  },
                  {
                    input: a.identity,
                    output: b.identity,
                    amount: inputRaw,
                    spread,
                    decimals: b.decimals,
                    offerDecimals: a.decimals,
                    cacheObservation,
                  },
                );
                await pay.fill(amount);
                await page
                  .waitForFunction(
                    () => window.__measurement1473?.done != null,
                    undefined,
                    { timeout: 15000, polling: 10 },
                  )
                  .catch(() => {});
                const result = await page.evaluate(() =>
                  window.__measurement1473.read(),
                );
                const transport = connections
                  .filter(
                    (row) =>
                      row.at >= nodeStart &&
                      row.input === a.identity &&
                      row.output === b.identity &&
                      row.amount === inputRaw,
                  )
                  .at(-1);
                const row = {
                  case_id: caseIdentity({
                    matrixIdentity, profile: PROFILE, viewport: view,
                    trial: cycle ? "retest" : "first", from, to, spread,
                    ...amountCase, raw: inputRaw,
                  }),
                  directional_group_index: group.index,
                  viewport: view,
                  trial: cycle ? "retest" : "first",
                  pair: `${from}->${to}`,
                  amount_class: amountCase.kind,
                  human_amount: amount,
                  raw_input: inputRaw,
                  selected_spread: spread,
                  ...result,
                  transport: transport
                    ? transport.reused
                      ? "reused"
                      : "new"
                    : result.attempts === 0
                      ? "browser_cache_hit"
                      : "unobserved",
                  http_protocol: transport?.protocol ?? null,
                };
                row.failed =
                  !row.classified ||
                  row.secondary_estimate_present ||
                  row.generic_validation_copy ||
                  !row.policy_opt_in ||
                  (row.useful && row.input_to_dom_ms > BROWSER_LATENCY_BUDGET_MS);
                if (!row.failed) delete row.diagnostics;
                receipt.samples.push(row);
                save();
                await page.waitForTimeout(500);
              }
            }
        await context.close();
      }
      await attestBackend();
    }
    const finalArtifacts = await attestArtifacts();
    receipt.artifact_hashes.push({ phase: "after", assets: finalArtifacts });
    if (JSON.stringify(finalArtifacts) !== JSON.stringify(initialArtifacts))
      throw new Error("Frontend artifacts changed during trial");
    if (receipt.samples.length !== receipt.expected_sample_count)
      throw new Error("Incomplete browser matrix");
    assertCaseCoverage(selectedCaseIds, receipt.samples.map(row => row.case_id));
    await ownedBrowser.close();
    ownedBrowser = null;
    receipt.completed = true;
    save();
    process.exitCode = receipt.overall === "FAIL" ? 1 : 0;
    console.log(
      JSON.stringify({
        overall: receipt.overall,
        samples: receipt.samples.length,
        latency: receipt.useful_latency,
        requests: quoteRequests,
      }),
    );
  }
  const elapsedTimer = setTimeout(() => {
    elapsedBudgetExpired = true;
    receipt.failure_class = "ElapsedTimeBudget";
    void ownedBrowser?.close().catch(() => {});
  }, ELAPSED_LIMIT_MS);
  elapsedTimer.unref();
  main()
    .catch(async (error) => {
      await ownedBrowser?.close().catch(() => {});
      receipt.failure_class = receipt.failure_class || error.name;
      save();
      console.error(
        JSON.stringify({
          overall: "FAIL",
          failure_class: receipt.failure_class,
        }),
      );
      process.exitCode = 1;
    })
    .finally(() => clearTimeout(elapsedTimer));
} catch {
  console.error(
    JSON.stringify({
      overall: "FAIL",
      failure_class: "PrerequisiteValidation",
    }),
  );
  process.exitCode = 1;
}
