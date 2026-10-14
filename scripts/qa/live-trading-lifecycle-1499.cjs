#!/usr/bin/env node
// Fixed anonymous read-only production cohort. Requires independent exact artifact attestation.
const fs = require("node:fs"),
  path = require("node:path"),
  crypto = require("node:crypto");
const { createRequire } = require("node:module");
const ROOT = path.resolve(__dirname, "../.."),
  rf = createRequire(path.join(ROOT, "frontend-dapp/package.json"));
const { chromium } = rf("@playwright/test"),
  { validate } = require("./live-quote-economics-1500.cjs");
const FE = "https://dex.cl8y.com",
  BE = "https://indexer.dex.cl8y.com",
  DURATION = 190000,
  QUOTE_BUDGET = 600,
  GLOBAL_BUDGET = 16 * 60000;
const SHA = process.env.EXPECTED_SOURCE_SHA,
  ATTEST = process.env.DEX_LIVE_ATTESTATION,
  OUT = process.env.DEX_LIVE_OUTPUT;
const SHARD = Number(process.env.DEX_LIVE_SHARD);
const {
  installInputClock,
  eligibleQuotesAtSnapshot,
} = require("./live-input-clock-1499.cjs");
const REASONS = {
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
const receipt = {
  issues: [1498, 1499, 1500],
  expected_revision: SHA,
  qa_measurement_version: "foreground_in_page_v2",
  shard: SHARD,
  shard_count: 3,
  expected_window_count: 4,
  cohort_maximum_windows: 12,
  shard_quote_progress_budget: 600,
  cohort_quote_progress_budget: 1800,
  concurrency_per_browser: 1,
  utc_start: new Date().toISOString(),
  read_only: true,
  signing: false,
  screenshots: false,
  measurement_clock: "original_input_event_through_two_render_frames",
  duration_per_case_ms: DURATION,
  browser_budget_ms: 750,
  socket_budget_ms: 500,
  api: [],
  windows: [],
  failures: [],
  completed: false,
  limits:
    "Anonymous six-direction selected cohort only; no funded/physical signing, independent oracle/contract attestation or server-side RPC census; monitored requests do not drive ingestion.",
};
let browser,
  quotes = 0,
  readCount = 0,
  rejectedQuoteAttempts = 0,
  routeReads = { solve: 0, progress: 0, best: 0 },
  blockedWrites = 0,
  blockedProvider = 0,
  blockedOther = 0;
const start = Date.now();
function ensure() {
  if (Date.now() - start > GLOBAL_BUDGET) throw Error("elapsed_budget");
}
function fail(code) {
  if (receipt.failures.length < 80) receipt.failures.push(code);
}
function save() {
  if (!OUT) return;
  fs.writeFileSync(
    OUT,
    JSON.stringify(
      {
        ...receipt,
        utc_end: new Date().toISOString(),
        quote_reads: quotes,
        rejected_quote_attempts: rejectedQuoteAttempts,
        route_reads: routeReads,
        read_count: readCount,
        blocked_writes: blockedWrites,
        blocked_provider: blockedProvider,
        blocked_other: blockedOther,
      },
      null,
      2,
    ) + "\n",
    { mode: 0o600 },
  );
  fs.chmodSync(OUT, 0o600);
}
async function read(url, limit = 2 * 1024 * 1024) {
  ensure();
  if (++readCount > 64) throw Error("read_budget");
  const response = await fetch(url, {
      redirect: "error",
      signal: AbortSignal.timeout(15000),
      headers: { "Cache-Control": "no-cache" },
    }),
    chunks = [];
  let size = 0;
  for await (const data of response.body) {
    size += data.length;
    if (size > limit) throw Error("body_bound");
    chunks.push(data);
  }
  return { status: response.status, bytes: Buffer.concat(chunks) };
}
const hash = (b) => crypto.createHash("sha256").update(b).digest("hex");
async function artifacts(attested) {
  const r = await read(FE + "/", 128 * 1024);
  if (r.status !== 200 || hash(r.bytes) !== attested.html_sha256)
    throw Error("root_html_mismatch");
  const paths = [
    ...r.bytes.toString().matchAll(/<script\b[^>]*\bsrc=["']([^"']+)["']/g),
  ].map((m) => m[1]);
  if (
    !paths.length ||
    paths.length > 8 ||
    new Set(paths).size !== paths.length ||
    paths.some(
      (p) =>
        !/^\/assets\/[-A-Za-z0-9_.]+\.js$/.test(p) &&
        !["/bootstrap/theme.js", "/bootstrap/trade-shell.js"].includes(p),
    )
  )
    throw Error("root_manifest");
  const actual = [];
  for (const p of paths) {
    const asset = await read(FE + p, 8 * 1024 * 1024);
    if (asset.status !== 200) throw Error("root_asset_read");
    actual.push({ asset: path.basename(p), sha256: hash(asset.bytes) });
  }
  if (
    JSON.stringify(actual) !==
    JSON.stringify(
      attested.root_scripts.map((r) => ({ asset: r.asset, sha256: r.sha256 })),
    )
  )
    throw Error("root_scripts_mismatch");
  return { html_sha256: hash(r.bytes), root_scripts: actual };
}
function raw(h, d) {
  const [w, f = ""] = h.split(".");
  return (
    BigInt(w) * 10n ** BigInt(d) +
    BigInt(f.padEnd(d, "0") || "0")
  ).toString();
}
function human(n, d) {
  const v = n.padStart(d + 1, "0");
  return d
    ? (v.slice(0, -d) + "." + v.slice(-d)).replace(/0+$/, "").replace(/\.$/, "")
    : v;
}
function metric(n) {
  if (n == null) return "Unavailable";
  return n && Math.abs(n) < 0.01
    ? n < 0
      ? ">-0.01%"
      : "<0.01%"
    : n.toFixed(2) + "%";
}
function params(c) {
  return {
    token_in: c.from,
    token_out: c.to,
    amount_in: raw(c.amount, c.inDecimals),
    max_spread: "0.05",
    max_maker_fills: "8",
    protection_policy: "quote_relative_v1",
    ...(c.native ? { pool_only: "true" } : {}),
  };
}
const SAFE_FAILURES = new Set([
  "book_binding",
  "attribution_sum",
  "backend_revision_mismatch",
  "body_bound",
  "book_bound",
  "book_cost",
  "book_output",
  "book_shape",
  "book_terms",
  "book_zero",
  "catalog_bound",
  "clunc_identity_unavailable",
  "counterfactual_budget",
  "counterfactual_output",
  "decimal_shape",
  "diagnostic_binding",
  "diagnostic_cohort",
  "diagnostic_sum",
  "elapsed_budget",
  "execution_binding",
  "explicit_revision_attestation_output_required",
  "final_backend_revision_mismatch",
  "hop_floor",
  "independent_attestation_mismatch",
  "market_ceiling",
  "market_spread",
  "operation_binding",
  "operation_path",
  "original_stage",
  "original_window",
  "output_floor",
  "pool_overflow",
  "projection_overflow",
  "quote_budget",
  "raw_bound",
  "raw_shape",
  "read_budget",
  "reserve_binding",
  "wrapper_binding",
  "operation_output",
  "response_history_bound",
  "root_asset_read",
  "root_html_mismatch",
  "root_manifest",
  "root_scripts_mismatch",
]);
function safeError(error) {
  if (SAFE_FAILURES.has(error?.message)) return error.message;
  return ["TimeoutError", "AbortError"].includes(error?.name)
    ? error.name
    : "probe_unavailable";
}
async function probe(c, phase) {
  if (quotes >= QUOTE_BUDGET) throw Error("quote_budget");
  quotes++;
  routeReads.solve++;
  const p = params(c),
    t = performance.now();
  try {
    const r = await read(BE + "/api/v1/route/solve?" + new URLSearchParams(p));
    const duration = performance.now() - t,
      b = JSON.parse(r.bytes),
      q = r.status === 200 ? validate(b, p, Date.now()) : null,
      reason = REASONS[b.error] === r.status ? b.error : null;
    const row = {
      case: c.label,
      phase,
      status: r.status,
      reason,
      elapsed_ms: duration,
      useful: !!q?.useful,
      ...q,
    };
    if (r.status === 200 && !q?.useful) fail("api_invalid_quote");
    if (!q && !reason) fail("api_unclassified_outcome");
    if (q?.useful && duration > 500) fail("socket_useful_over500");
    receipt.api.push(row);
  } catch (e) {
    receipt.api.push({
      case: c.label,
      phase,
      useful: false,
      failure: safeError(e),
    });
    fail("api_probe_failed");
  }
}
async function setup(page, c) {
  await page.goto(FE + "/?" + new URLSearchParams({ from: c.from, to: c.to }), {
    waitUntil: "domcontentloaded",
    timeout: 30000,
  });
  const ack = page.getByRole("checkbox", { name: /I understand these risks/ });
  if (await ack.isVisible().catch(() => false)) {
    await ack.check();
    await page.getByRole("button", { name: "Continue to the app" }).click();
  }
  const pay = page.getByTestId("swap-you-pay-amount");
  await pay.waitFor({ timeout: 20000 });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByTestId("swap-slippage-preset-5").click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.locator("#swap-slippage-settings").waitFor({ state: "hidden" });
  await pay.fill("0");
  await page.getByTestId("swap-details").evaluate((n) => {
    n.open = true;
  });
  await pay.evaluate((n) => {
    const r = n.getBoundingClientRect(),
      b = document
        .querySelector('[data-testid="swap-you-receive"]')
        .getBoundingClientRect();
    scrollBy({
      top:
        (Math.min(r.top, b.top) + Math.max(r.bottom, b.bottom)) / 2 -
        innerHeight / 2,
      behavior: "instant",
    });
  });
  await page.waitForFunction(() =>
    ["swap-you-pay-amount", "swap-you-receive"].every((id) => {
      const r = document
        .querySelector(`[data-testid="${id}"]`)
        ?.getBoundingClientRect();
      return (
        r?.width > 0 && r.height > 0 && r.top < innerHeight && r.bottom > 0
      );
    }),
  );
  return pay;
}
async function windowCase(c, view, viewport) {
  ensure();
  const context = await browser.newContext({
      viewport,
      serviceWorkers: "block",
      reducedMotion: "reduce",
    }),
    page = await context.newPage(),
    events = [],
    pending = new Set();
  const row = {
    case: c.label,
    viewport: view,
    input_raw: params(c).amount_in,
    samples: 0,
    transitions: [],
    responses: [],
    failures: [],
    failure_details: [],
    first_useful_ms: null,
    first_dom_ms: null,
    response_probe_failures: [],
    completed: false,
  };
  let inputAt = 0,
    baseline = null,
    lastSignature = null,
    latest = null,
    windowEnded = false;
  const problem = (code) => {
    if (!row.failures.includes(code)) row.failures.push(code);
  };
  await context.routeWebSocket("**/*", (s) => {
    blockedOther++;
    s.close();
  });
  await context.route("**/*", async (route) => {
    const r = route.request(),
      u = new URL(r.url());
    if (!["GET", "HEAD", "OPTIONS"].includes(r.method())) {
      blockedWrites++;
      return route.abort();
    }
    if (u.protocol === "data:") return route.continue();
    const provider = /\/cosmwasm\/|\/cosmos\/|\/terra\/|\/rpc(?:\/|$)/.test(
      u.pathname,
    );
    if (provider) {
      blockedProvider++;
      return route.abort();
    }
    if (
      ![FE, BE].includes(u.origin) &&
      !["image", "font", "stylesheet"].includes(r.resourceType())
    ) {
      blockedOther++;
      return route.abort();
    }
    if (u.origin === BE && u.searchParams.has("trader")) {
      blockedWrites++;
      return route.abort();
    }
    if (u.pathname.startsWith("/api/v1/route/solve")) {
      if (quotes >= QUOTE_BUDGET) {
        rejectedQuoteAttempts++;
        problem("quote_budget");
        return route.abort();
      }
      quotes++;
      routeReads[
        u.pathname.endsWith("/progress")
          ? "progress"
          : u.pathname.endsWith("/best")
            ? "best"
            : "solve"
      ]++;
    }
    return route.continue();
  });
  page.on("pageerror", () => problem("browser_page_error"));
  page.on("response", (response) => {
    const url = new URL(response.url());
    if (url.origin !== BE || url.pathname !== "/api/v1/route/solve") return;
    const work = (async () => {
      const p = Object.fromEntries(url.searchParams);
      if (
        p.token_in !== c.from ||
        p.token_out !== c.to ||
        p.amount_in !== params(c).amount_in
      )
        return;
      const content = await response.body();
      if (content.length > 2 * 1024 * 1024) throw Error("body_bound");
      const body = JSON.parse(content),
        status = response.status(),
        at = Date.now();
      let q = null,
        reason = REASONS[body.error] === status ? body.error : null;
      try {
        if (status === 200) q = validate(body, p, at);
      } catch (error) {
        problem("browser_client_validation:" + safeError(error));
      }
      const e = { status, reason, at_ms: at - inputAt, ...q };
      events.push({ ...e, absolute_at: at });
      if (events.length > 120) throw Error("response_history_bound");
      latest = events.at(-1);
      row.responses.push(e);
      if (status !== 200 && !reason) problem("browser_unclassified_outcome");
    })().catch((error) => {
      const requestFailure = response.request().failure()?.errorText;
      const cancelled = requestFailure === "net::ERR_ABORTED";
      const closing = windowEnded;
      row.response_probe_failures.push({
        phase: closing ? "closing" : "observing",
        at_ms: Date.now() - inputAt,
        class: cancelled ? "request_cancelled" : safeError(error),
      });
      if (!closing && !cancelled)
        problem("observing_response_probe_unavailable");
    });
    pending.add(work);
    work.finally(() => pending.delete(work));
  });
  try {
    const pay = await setup(page, c);
    await page.bringToFront();
    inputAt = Date.now();
    await page.evaluate(installInputClock);
    await pay.fill(c.amount);
    inputAt = await page.evaluate(
      () => window.__liveClock1499.started_epoch_ms,
    );
    const deadline = Date.now() + DURATION;
    while (Date.now() < deadline) {
      ensure();
      if (quotes >= QUOTE_BUDGET) {
        problem("quote_budget");
        break;
      }
      const state = await page.evaluate(() => {
        const sampleEpochMs = Date.now();
        const ids = [
            "swap-you-pay-amount",
            "swap-you-receive",
            "swap-quote-summary",
            "swap-quote-status",
          ],
          nodes = ids.map((id) =>
            document.querySelector(`[data-testid="${id}"]`),
          );
        return {
          value: nodes[1]?.value ?? null,
          loading: !!document.querySelector(
            '[data-testid="swap-price-data-loading"]',
          ),
          unsupported: (
            document.querySelector('[data-testid="swap-quote-status"]')
              ?.textContent || ""
          ).includes("not supported by current pricing"),
          slippage:
            document.querySelector('[data-testid="swap-order-slippage"]')
              ?.textContent || "",
          fee:
            document.querySelector('[data-testid="swap-trading-fee"]')
              ?.textContent || "",
          details: [...document.querySelectorAll("details > summary")].filter(
            (n) => n.textContent.trim() === "DETAILS",
          ).length,
          detailsOpen:
            document.querySelector('[data-testid="swap-details"]')?.open ===
            true,
          networkFees: document.querySelectorAll(
            '[data-testid="swap-network-fee"]',
          ).length,
          overflow: document.documentElement.scrollWidth > innerWidth + 1,
          geometry: nodes.map((n) => {
            if (!n) return null;
            const r = n.getBoundingClientRect();
            return {
              x: r.x + scrollX,
              y: r.y + scrollY,
              width: r.width,
              height: r.height,
            };
          }),
          sample_epoch_ms: sampleEpochMs,
          visibility: document.visibilityState,
          hidden: document.hidden,
          clockCandidates: window.__liveClock1499?.candidates ?? [],
          inputAge:
            window.__liveInputStarted == null
              ? null
              : performance.now() - window.__liveInputStarted,
        };
      });
      row.samples++;
      if (state.visibility !== "visible" || state.hidden)
        problem("nonforeground_observation");
      const candidate = state.clockCandidates.find((clock) =>
        events.some(
          (e) =>
            e.useful &&
            human(e.output, c.outDecimals) === clock.value &&
            e.observed <= clock.paint_epoch_ms &&
            e.expires > clock.paint_epoch_ms,
        ),
      );
      if (candidate && row.first_useful_ms == null) {
        row.first_dom_ms = candidate.first_dom_ms;
        row.first_useful_ms = candidate.input_to_paint_ms;
        if (row.first_useful_ms > 750) problem("browser_useful_over750");
        receipt.active_clock = {
          case: c.label,
          viewport: view,
          first_dom_ms: row.first_dom_ms,
          first_useful_ms: row.first_useful_ms,
          visibility: state.visibility,
        };
        process.stderr.write(
          JSON.stringify({ event: "first_useful", ...receipt.active_clock }) +
            "\n",
        );
        save();
      }
      if (!baseline && state.geometry.every(Boolean)) baseline = state.geometry;
      if (
        baseline &&
        state.geometry.some(
          (r, i) =>
            !r ||
            ["x", "y", "width", "height"].some(
              (k) => Math.abs(r[k] - baseline[i][k]) > 1,
            ),
        )
      )
        problem("unstable_geometry");
      if (
        state.details !== 1 ||
        !state.detailsOpen ||
        state.networkFees !== 1 ||
        state.overflow
      )
        problem("layout_contract");
      const eligible = eligibleQuotesAtSnapshot(events, state.sample_epoch_ms),
        matching = eligible.find(
          (e) => human(e.output, c.outDecimals) === state.value,
        );
      if (!state.value && eligible.length) {
        problem("unexpired_quote_blank");
        if (row.failure_details.length < 32)
          row.failure_details.push({
            code: "unexpired_quote_blank",
            sample_epoch_ms: state.sample_epoch_ms,
            input_age_ms: state.inputAge,
            latest_reason: latest?.reason ?? null,
            original_expiries: eligible.map((e) => e.expires),
          });
      }
      if (
        state.value &&
        !matching &&
        events.some((e) => e.useful) &&
        pending.size === 0 &&
        (!latest || state.sample_epoch_ms - latest.absolute_at > 250)
      ) {
        problem("unvalidated_or_expired_display");
        if (row.failure_details.length < 32)
          row.failure_details.push({
            code: "unvalidated_or_expired_display",
            sample_epoch_ms: state.sample_epoch_ms,
            evaluated_epoch_ms: Date.now(),
            input_age_ms: state.inputAge,
            value: state.value,
            latest_status: latest?.status ?? null,
            latest_reason: latest?.reason ?? null,
            same_output_quotes: events
              .filter(
                (e) =>
                  e.useful && human(e.output, c.outDecimals) === state.value,
              )
              .map((e) => ({
                observed_at_ms: e.observed,
                expires_at_ms: e.expires,
                expiry_delta_ms: state.sample_epoch_ms - e.expires,
              })),
          });
      }
      if (state.unsupported && latest?.reason !== "unsupported_semantics")
        problem("unsupported_without_typed_negative");
      if (matching) {
        if (
          matching.metrics &&
          (state.slippage !== "Slippage " + metric(matching.slippage) ||
            state.fee !== "Fee " + metric(matching.fee))
        )
          problem("metric_display_mismatch");
      }
      const signature = JSON.stringify([
        state.value,
        state.loading,
        state.unsupported,
        state.slippage,
        state.fee,
      ]);
      if (signature !== lastSignature) {
        if (row.transitions.length < 120)
          row.transitions.push({
            at_ms: state.inputAge,
            sample_epoch_ms: state.sample_epoch_ms,
            value: state.value,
            loading: state.loading,
            unsupported: state.unsupported,
            slippage: state.slippage,
            fee: state.fee,
            geometry: state.geometry,
          });
        lastSignature = signature;
      }
      await page.waitForTimeout(200);
    }
    windowEnded = true;
    await page.evaluate(() => window.__liveClock1499?.stop());
    await Promise.all([...pending]);
    row.original_input_epoch_ms = inputAt;
    row.elapsed_ms = Date.now() - inputAt;
    row.completed = row.elapsed_ms >= 185000;
    row.original_observation_windows = events
      .filter((e) => e.useful)
      .map((e) => ({ observed_at_ms: e.observed, expires_at_ms: e.expires }));
    if (row.first_useful_ms == null) problem("no_useful_browser_quote");
    if (row.elapsed_ms < 185000) problem("incomplete_expiry_windows");
  } catch (e) {
    problem(safeError(e));
  } finally {
    windowEnded = true;
    receipt.windows.push(row);
    save();
    await context.close();
    process.stderr.write(
      JSON.stringify({
        case: c.label,
        viewport: view,
        completed: row.completed,
        failures: row.failures,
      }) + "\n",
    );
  }
}
async function main() {
  if (
    ![0, 1, 2].includes(SHARD) ||
    !/^[0-9a-f]{40}$/.test(SHA || "") ||
    !ATTEST ||
    !OUT ||
    fs.statSync(ATTEST).size > 65536
  )
    throw Error("explicit_revision_attestation_output_required");
  const att = JSON.parse(fs.readFileSync(ATTEST));
  if (
    att.overall !== "PASS" ||
    att.expected_revision !== SHA ||
    att.frontend?.source_sha !== SHA ||
    att.backend?.git_sha !== SHA ||
    att.backend_deployment?.source_sha !== SHA ||
    att.frontend?.deployment_status !== "finished" ||
    att.backend_deployment?.deployment_status !== "finished"
  )
    throw Error("independent_attestation_mismatch");
  receipt.qa_helper_hashes = [
    "live-trading-lifecycle-1499.cjs",
    "live-input-clock-1499.cjs",
    "live-quote-economics-1500.cjs",
  ].map((file) => ({
    file,
    sha256: hash(fs.readFileSync(path.join(__dirname, file))),
  }));
  receipt.artifact_before = await artifacts(att.frontend.artifact);
  const health = await read(BE + "/health", 4096);
  if (JSON.parse(health.bytes).git_sha !== SHA)
    throw Error("backend_revision_mismatch");
  const catalog = JSON.parse((await read(BE + "/api/v1/tokens")).bytes);
  if (!Array.isArray(catalog) || catalog.length > 4096)
    throw Error("catalog_bound");
  const clunc =
    process.env.DEX_LIVE_CLUNC ||
    catalog
      .filter((t) => t.symbol === "cLUNC")
      .map((t) => t.contract_address)
      .filter(Boolean)
      .filter((v, i, a) => a.indexOf(v) === i)
      .find((v, i, a) => a.length === 1);
  if (!/^terra1[a-z0-9]{38,90}$/.test(clunc || ""))
    throw Error("clunc_identity_unavailable");
  const assets = JSON.parse(
    fs.readFileSync(path.join(ROOT, "docs/qa/issue-1473/matrix.json")),
  ).assets;
  assets.cLUNC = { identity: clunc, decimals: 6 };
  const groups = [
    ["USTC", "CL8Y", "132"],
    ["CL8Y", "USTC", "132"],
    ["CL8Y", "cLUNC", "13"],
    ["cLUNC", "CL8Y", "13"],
    ["UST1", "CL8Y", "1"],
    ["CL8Y", "UST1", "1"],
  ];
  const cases = groups
    .slice(SHARD * 2, SHARD * 2 + 2)
    .map(([a, b, amount]) => ({
      label: a + "->" + b,
      from: assets[a].identity,
      to: assets[b].identity,
      inDecimals: assets[a].decimals,
      outDecimals: assets[b].decimals,
      amount,
      native: a === "USTC" || b === "USTC",
    }));
  for (const c of cases) {
    await probe(c, "initialized_first");
    await probe(c, "initialized_retest");
  }
  browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: [
      "--no-sandbox",
      "--disable-gpu",
      "--disable-dev-shm-usage",
      "--disable-background-timer-throttling",
      "--disable-renderer-backgrounding",
    ],
  });
  const tasks = cases.flatMap((c) =>
    [
      ["desktop", { width: 1280, height: 900 }],
      ["phone", { width: 390, height: 844 }],
    ].map(
      ([view, viewport]) =>
        () =>
          windowCase(c, view, viewport),
    ),
  );
  for (const task of tasks) {
    if (quotes >= QUOTE_BUDGET) {
      fail("quote_budget");
      break;
    }
    await task();
  }
  receipt.artifact_after = await artifacts(att.frontend.artifact);
  const finalHealth = JSON.parse((await read(BE + "/health", 4096)).bytes);
  if (finalHealth.git_sha !== SHA)
    throw Error("final_backend_revision_mismatch");
  receipt.completed =
    receipt.windows.length === 4 && receipt.windows.every((w) => w.completed);
  if (blockedWrites || blockedProvider) fail("prohibited_request_attempts");
  receipt.overall =
    receipt.failures.length ||
    receipt.windows.some((w) => w.failures.length) ||
    !receipt.completed
      ? "FAIL"
      : "PASS";
}
main()
  .catch((e) => {
    fail(safeError(e));
    receipt.overall = "FAIL";
  })
  .finally(async () => {
    if (browser) await browser.close();
    save();
    process.stdout.write(
      JSON.stringify({
        overall: receipt.overall,
        completed: receipt.completed,
        window_count: receipt.windows.length,
        failures: receipt.failures,
      }) + "\n",
    );
    process.exitCode = receipt.overall === "PASS" ? 0 : 1;
  });
