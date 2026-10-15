#!/usr/bin/env node
// Fixed anonymous read-only production cohort. Requires independent exact artifact attestation.
const fs = require("node:fs"),
  path = require("node:path"),
  crypto = require("node:crypto");
const { createRequire } = require("node:module");
const ROOT = path.resolve(__dirname, "../.."),
  rf = createRequire(path.join(ROOT, "frontend-dapp/package.json"));
const { chromium } = rf("@playwright/test"),
  {
    validate,
    originalWindowDiagnostics,
  } = require("./live-quote-economics-1500.cjs");
const {
  CONTEXT_QUOTE_MAXIMUM,
  quoteBudgetProfile,
  contextQuoteAllowance,
  quoteBudgetExhausted,
} = require("./live-quote-budget-1505.cjs");
const NATIVE_1505 = process.env.DEX_LIVE_ISSUE === "1505";
const BUDGET_PROFILE = quoteBudgetProfile(NATIVE_1505);
const FE = "https://dex.cl8y.com",
  BE = "https://indexer.dex.cl8y.com",
  DURATION = 190000,
  QUOTE_BUDGET = BUDGET_PROFILE.quoteProgressBudget,
  GLOBAL_BUDGET = 16 * 60000;
const SHA = process.env.EXPECTED_SOURCE_SHA,
  ATTEST = process.env.DEX_LIVE_ATTESTATION,
  OUT = process.env.DEX_LIVE_OUTPUT;
const SHARD = Number(process.env.DEX_LIVE_SHARD);
const {
  installInputClock,
  eligibleQuotesAtSnapshot,
  displayedQuoteRetention,
  matchingDisplayedQuotes,
  updateDisplayedQuoteOwners,
  nativeResponseEpoch,
  recordWindowQuoteOutcome,
  drainResponseProbes,
  isOwnedTeardownCancellation,
} = require("./live-input-clock-1499.cjs");
const {
  installReadGuard,
  nativeBlockRules,
} = require("./live-read-guard-1499.cjs");
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
  issues: NATIVE_1505 ? [1505] : [1498, 1499, 1500],
  expected_revision: SHA,
  qa_measurement_version: BUDGET_PROFILE.measurementVersion,
  shard: SHARD,
  shard_count: BUDGET_PROFILE.shardCount,
  expected_window_count: 4,
  cohort_maximum_windows: NATIVE_1505 ? 4 : 12,
  shard_quote_progress_budget: QUOTE_BUDGET,
  cohort_quote_progress_budget: BUDGET_PROFILE.cohortQuoteProgressBudget,
  context_quote_progress_maximum: CONTEXT_QUOTE_MAXIMUM,
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
    (NATIVE_1505
      ? "Anonymous native bidirectional desktop/phone cohort only; "
      : "Anonymous six-direction selected cohort only; ") +
    "no funded/physical signing, independent oracle/contract attestation or server-side RPC census; monitored requests do not drive ingestion.",
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
  return {
    status: response.status,
    bytes: Buffer.concat(chunks),
    request_id: safeRequestId(response.headers.get("x-quote-request-id")),
  };
}
function safeRequestId(value) {
  return /^[0-9a-f]{16}$/.test(value || "") ? value : null;
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
  "native_response_clock_unavailable",
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
  let invalidWindow = null;
  try {
    const r = await read(BE + "/api/v1/route/solve?" + new URLSearchParams(p));
    const duration = performance.now() - t,
      b = JSON.parse(r.bytes),
      reason = REASONS[b.error] === r.status ? b.error : null;
    let q = null;
    if (r.status === 200) {
      const receivedAt = Date.now();
      try {
        q = validate(b, p, receivedAt);
      } catch (error) {
        if (error?.message === "original_window")
          invalidWindow = {
            ...originalWindowDiagnostics(b.execution_metadata, receivedAt),
            request_id: r.request_id,
          };
        throw error;
      }
    }
    const row = {
      case: c.label,
      phase,
      status: r.status,
      request_id: r.request_id,
      reason,
      elapsed_ms: duration,
      useful: !!q?.useful,
      ...q,
    };
    if (r.status === 200 && !q?.useful) fail("api_invalid_quote");
    if (!q && !reason) fail("api_unclassified_outcome");
    if (q?.useful && duration > 500) fail("socket_useful_over500");
    if (NATIVE_1505 && !q?.useful) fail("native_socket_not_useful");
    if (NATIVE_1505 && !r.request_id) fail("quote_request_id_unavailable");
    receipt.api.push(row);
  } catch (e) {
    receipt.api.push({
      case: c.label,
      phase,
      useful: false,
      failure: safeError(e),
      ...(invalidWindow ? { original_window: invalidWindow } : {}),
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
    quote_availability: {
      responses: 0,
      useful: 0,
      not_useful: 0,
      teardown_cancelled: 0,
    },
    failures: [],
    failure_details: [],
    display_ownership_qualifications: [],
    first_useful_ms: null,
    first_dom_ms: null,
    first_observed_dom_ms: null,
    first_qualified_dom_ms: null,
    response_probe_failures: [],
    completed: false,
    active_window_completed: false,
    active_window_elapsed_ms: null,
    active_window_stop_reason: null,
  };
  let inputAt = 0,
    baseline = null,
    lastSignature = null,
    latest = null,
    windowEnded = false,
    contextClosing = false,
    activeStarted = false,
    stoppedForBudget = false,
    displayedOwnership = { signature: null, owners: [], unavailable: false };
  const problem = (code) => {
    if (!row.failures.includes(code)) row.failures.push(code);
  };
  const contextQuoteStart = quotes;
  const contextAllowance = contextQuoteAllowance(QUOTE_BUDGET, quotes);
  await context.addInitScript(installReadGuard, {
    frontend: FE,
    backend: BE,
    remaining: contextAllowance,
  });
  const network = await context.newCDPSession(page);
  await network.send("Network.enable");
  await network.send("Network.setBlockedURLs", nativeBlockRules(FE, BE));
  // Passive accounting only; no Fetch.enable/request-paused roundtrip for normal GETs.
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (
      url.pathname.startsWith("/api/v1/route/solve") &&
      ["GET", "HEAD", "OPTIONS"].includes(request.method())
    ) {
      quotes++;
      routeReads[
        url.pathname.endsWith("/progress")
          ? "progress"
          : url.pathname.endsWith("/best")
            ? "best"
            : "solve"
      ]++;
      if (quotes > QUOTE_BUDGET) problem("quote_budget");
    }
    if (!["GET", "HEAD", "OPTIONS"].includes(request.method()))
      problem("actual_write_violation");
    if (/\/(?:cosmwasm|cosmos|terra|rpc)(?:\/|$)/.test(url.pathname))
      problem("actual_provider_violation");
  });
  page.on("pageerror", () => problem("browser_page_error"));
  page.on("response", (response) => {
    const url = new URL(response.url());
    if (url.origin !== BE || url.pathname !== "/api/v1/route/solve") return;
    let selected = false,
      recorded = false,
      readingBody = false;
    const work = (async () => {
      const p = Object.fromEntries(url.searchParams);
      if (
        p.token_in !== c.from ||
        p.token_out !== c.to ||
        p.amount_in !== params(c).amount_in
      )
        return;
      selected = true;
      readingBody = true;
      const content = await response.body();
      readingBody = false;
      if (content.length > 2 * 1024 * 1024) throw Error("body_bound");
      const body = JSON.parse(content),
        status = response.status(),
        at = nativeResponseEpoch(response.request().timing()),
        deliveredAt = Date.now();
      let q = null,
        reason = REASONS[body.error] === status ? body.error : null;
      try {
        if (status === 200) q = validate(body, p, at);
      } catch (error) {
        problem("browser_client_validation:" + safeError(error));
        if (
          error?.message === "original_window" &&
          row.failure_details.length < 32
        )
          row.failure_details.push({
            code: "browser_client_validation:original_window",
            ...originalWindowDiagnostics(body.execution_metadata, at),
            request_id: safeRequestId(response.headers()["x-quote-request-id"]),
          });
      }
      const e = {
        status,
        request_id: safeRequestId(response.headers()["x-quote-request-id"]),
        reason,
        at_ms: at - inputAt,
        diagnostic_delivery_lag_ms: Math.max(0, deliveredAt - at),
        ...q,
      };
      events.push({ ...e, absolute_at: at });
      events.sort((a, b) => a.absolute_at - b.absolute_at);
      if (events.length > 120) throw Error("response_history_bound");
      latest = events.at(-1);
      recordWindowQuoteOutcome(row, e, NATIVE_1505);
      recorded = true;
      if (NATIVE_1505 && !e.request_id) problem("quote_request_id_unavailable");
      if (status !== 200 && !reason) problem("browser_unclassified_outcome");
    })().catch((error) => {
      const requestFailure = response.request().failure()?.errorText;
      const cancelled = requestFailure === "net::ERR_ABORTED";
      const intentionalCancellation = isOwnedTeardownCancellation(
        response.status(),
        readingBody,
        contextClosing,
        requestFailure,
        error,
      );
      if (selected && !recorded)
        recordWindowQuoteOutcome(
          row,
          {
            status: response.status(),
            request_id: safeRequestId(response.headers()["x-quote-request-id"]),
            useful: false,
            failure: intentionalCancellation
              ? "request_cancelled"
              : safeError(error),
          },
          NATIVE_1505,
          intentionalCancellation,
        );
      const closing = windowEnded;
      row.response_probe_failures.push({
        phase: closing ? "closing" : "observing",
        at_ms: Date.now() - inputAt,
        class:
          cancelled || intentionalCancellation
            ? "request_cancelled"
            : safeError(error),
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
    activeStarted = true;
    const deadline = Date.now() + DURATION;
    while (Date.now() < deadline) {
      ensure();
      if (quoteBudgetExhausted(QUOTE_BUDGET, quotes, contextQuoteStart, contextAllowance)) {
        problem("quote_budget");
        stoppedForBudget = true;
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
          read_guard: window.__liveReadGuard1499,
          sample_epoch_ms: sampleEpochMs,
          visibility: document.visibilityState,
          hidden: document.hidden,
          clockCandidates: window.__liveClock1499?.candidates ?? [],
          clockDomObservations: window.__liveClock1499?.dom_observations ?? [],
          clockDiagnostics: {
            long_task_observer_available:
              window.__liveClock1499?.long_task_observer_available,
            long_tasks: window.__liveClock1499?.long_tasks ?? [],
            frame_gaps: window.__liveClock1499?.frame_gaps ?? [],
          },
          inputAge:
            window.__liveInputStarted == null
              ? null
              : performance.now() - window.__liveInputStarted,
        };
      });
      row.samples++;
      row.clock_diagnostics = state.clockDiagnostics;
      if (
        row.first_observed_dom_ms == null &&
        state.clockDomObservations.length
      )
        row.first_observed_dom_ms = state.clockDomObservations[0].at_ms;
      if (state.read_guard?.rejected) problem("quote_budget");
      if (
        state.read_guard?.writes ||
        state.read_guard?.providers ||
        state.read_guard?.actors ||
        state.read_guard?.workers
      )
        problem("blocked_unsafe_request_attempt");
      if (state.visibility !== "visible" || state.hidden)
        problem("nonforeground_observation");
      const candidate = state.clockCandidates.find(
        (clock) =>
          matchingDisplayedQuotes(
            eligibleQuotesAtSnapshot(events, clock.paint_epoch_ms).filter(
              (e) => e.absolute_at <= clock.paint_epoch_ms,
            ),
            clock,
            (raw) => human(raw, c.outDecimals),
            metric,
          ).length > 0,
      );
      if (candidate && row.first_useful_ms == null) {
        row.first_dom_ms = candidate.first_dom_ms;
        row.first_qualified_dom_ms = candidate.first_dom_ms;
        row.first_useful_ms = candidate.input_to_paint_ms;
        row.first_clock_diagnostics = state.clockDiagnostics;
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
        matchingOwners = matchingDisplayedQuotes(
          eligible,
          state,
          (raw) => human(raw, c.outDecimals),
          metric,
        ),
        matching = matchingOwners[0];
      const displayedSignature = state.value
        ? JSON.stringify([state.value, state.slippage, state.fee])
        : null;
      displayedOwnership = updateDisplayedQuoteOwners(
        displayedOwnership,
        displayedSignature,
        matchingOwners,
      );
      const displayedOwners = displayedOwnership.owners;
      const retention = displayedQuoteRetention(
        displayedOwners,
        events,
        state.sample_epoch_ms,
        displayedOwnership.unavailable,
      );
      if (
        !state.value &&
        ["ambiguous", "unavailable"].includes(retention) &&
        row.display_ownership_qualifications.length < 32
      )
        row.display_ownership_qualifications.push({
          sample_epoch_ms: state.sample_epoch_ms,
          scope:
            retention === "unavailable"
              ? "owner_bound_unavailable"
              : "identical_display_cohort_ownership_unavailable",
          possible_original_expiries: displayedOwners.map((e) => e.expires),
        });
      if (!state.value && retention === "unexpired") {
        problem("unexpired_quote_blank");
        if (row.failure_details.length < 32)
          row.failure_details.push({
            code: "unexpired_quote_blank",
            sample_epoch_ms: state.sample_epoch_ms,
            input_age_ms: state.inputAge,
            latest_reason: latest?.reason ?? null,
            original_expiries: displayedOwners.map((e) => e.expires),
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
      if (
        state.value &&
        !matching &&
        eligible.some((e) => human(e.output, c.outDecimals) === state.value)
      )
        problem("metric_display_mismatch");
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
    // Only natural exhaustion of the active duration completes a window. Setup
    // and subsequent response-drain time cannot complete an early budget exit.
    row.active_window_elapsed_ms = Date.now() - inputAt;
    row.active_window_completed = !stoppedForBudget;
    row.active_window_stop_reason = stoppedForBudget ? "quote_budget" : "duration";
    const guard = await page.evaluate(() => window.__liveReadGuard1499);
    row.read_guard = guard;
    blockedWrites += guard?.writes ?? 0;
    blockedProvider += guard?.providers ?? 0;
    blockedOther += (guard?.unexpected ?? 0) + (guard?.workers ?? 0);
    rejectedQuoteAttempts += guard?.rejected ?? 0;
    windowEnded = true;
    await page.evaluate(() => window.__liveClock1499?.stop());
    row.response_drain = {
      limit_ms: 5000,
      settled: await drainResponseProbes(pending, 5000),
    };
    row.original_input_epoch_ms = inputAt;
    row.elapsed_ms = Date.now() - inputAt;
    row.completed = row.active_window_completed;
    row.original_observation_windows = events
      .filter((e) => e.useful)
      .map((e) => ({ observed_at_ms: e.observed, expires_at_ms: e.expires }));
    if (row.first_useful_ms == null) problem("no_useful_browser_quote");
  } catch (e) {
    row.completed = false;
    row.active_window_stop_reason ??= activeStarted ? "active_exception" : "setup_exception";
    if (activeStarted && row.active_window_elapsed_ms == null)
      row.active_window_elapsed_ms = Date.now() - inputAt;
    problem(safeError(e));
  } finally {
    if (!row.completed) problem("incomplete_expiry_windows");
    windowEnded = true;
    contextClosing = true;
    await context
      .close()
      .catch(() => problem("owned_context_close_unavailable"));
    // Flush deliberate-close cancellations into their own counted scope before saving.
    if (!(await drainResponseProbes(pending, 1000)))
      problem("teardown_response_probe_unavailable");
    receipt.windows.push(row);
    save();
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
    (NATIVE_1505 && SHARD !== 0) ||
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
    "live-read-guard-1499.cjs",
    "live-quote-budget-1505.cjs",
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
