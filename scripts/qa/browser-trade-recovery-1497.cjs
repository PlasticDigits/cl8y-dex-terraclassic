#!/usr/bin/env node
// Actual deployed assets; explicitly injected consumer failures, never backend outage evidence.
const fs = require("node:fs");
const path = require("node:path");
const { createHash } = require("node:crypto");
const { createRequire } = require("node:module");
const { spawnSync } = require("node:child_process");
const root = path.resolve(__dirname, "../..");
const { chromium, expect } = createRequire(
  path.join(root, "frontend-dapp/package.json"),
)("@playwright/test");
const args = process.argv.slice(2);
const option = (name, fallback) =>
  args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const directory = "/tmp/dex-1497-1500";
const output = option(
  "--output",
  `${directory}/live-trade-recovery-first.json`,
);
const attestation = JSON.parse(
  fs.readFileSync(
    option("--attestation", `${directory}/attestation-final.json`),
  ),
);
const artifacts = JSON.parse(
  fs.readFileSync(
    option("--artifacts", `${directory}/live-trade-public-artifacts.json`),
  ),
);
const entry = JSON.parse(
  fs.readFileSync(
    option("--entry-receipt", `${directory}/live-trade-first.json`),
  ),
);
const pair = entry.trials
  .find(
    (t) =>
      t.verdict === "PASS" &&
      /^\/trade\/terra1[a-z0-9]{38,64}$/.test(t.pathname ?? ""),
  )
  ?.pathname.split("/")
  .pop();
if (
  attestation.overall !== "PASS" ||
  !/^[a-f0-9]{40}$/.test(attestation.frontend?.source_sha ?? "") ||
  artifacts.verdict !== "PASS" ||
  !pair
)
  throw new Error("Independent public deployment/catalog evidence is required");
const origin = "https://dex.cl8y.com";
const indexerOrigin = "https://indexer.dex.cl8y.com";
const expectedAssets = new Map(
  artifacts.artifacts
    .filter((a) => a.path.endsWith(".js"))
    .map((a) => [a.path, a.sha256]),
);
const tradeAsset = [...expectedAssets.keys()].find((p) =>
  /\/TradePage-[^/]+\.js$/.test(p),
);
const LIMIT_API_READS = 500;
const LIMIT_DOCUMENTS_PER_CASE = 5;
const receivedAssets = new Map();
let totalApiReads = 0;
const receipt = {
  issue: 1497,
  schema: 1,
  started_at_utc: new Date().toISOString(),
  scope:
    "Read-only disconnected Trade consumer recovery with actual attested deployed assets; synthetic GET API503s, browser offline mode and TradePage404s are declared injections, not real backend/provider failures. No connected-wallet or transaction acceptance.",
  deployed_frontend_revision: attestation.frontend.source_sha,
  independent_attestation_utc: attestation.utc,
  qa_checkout_revision: spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim(),
  qa_checkout_dirty: !!spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
  }).stdout.trim(),
  qa_helper_sha256: createHash("sha256")
    .update(fs.readFileSync(__filename))
    .digest("hex"),
  api_read_cap: LIMIT_API_READS,
  document_cap_per_case: LIMIT_DOCUMENTS_PER_CASE,
  trials: [],
  served_assets: [],
  verdict: "FAIL",
};
async function acknowledge(page) {
  const checkbox = page.getByRole("checkbox", {
    name: /I understand these risks/,
  });
  if (await checkbox.isVisible()) {
    await checkbox.check();
    await page.getByRole("button", { name: "Continue to the app" }).click();
  }
}
async function ready(page, factory) {
  await expect(page.getByTestId("trade-page-heading")).toBeVisible({
    timeout: 30000,
  });
  await acknowledge(page);
  await expect(page.getByTestId("trade-order-tab-market")).toBeVisible({
    timeout: 30000,
  });
  await expect(page.getByTestId("trade-order-tab-limit")).toBeVisible();
  const pathname = new URL(page.url()).pathname;
  if (!/^\/trade\/terra1[a-z0-9]{38,64}$/.test(pathname))
    throw new Error("Trade destination was not preserved");
  await expect
    .poll(() => factory.has(pathname.split("/").pop()), { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("swap-you-pay-amount")).toHaveCount(0);
  return pathname;
}
async function setup(browser, width) {
  const context = await browser.newContext({
    viewport: { width, height: 900 },
  });
  const page = await context.newPage();
  await page.bringToFront();
  page.setDefaultTimeout(15000);
  const state = {
    catalogFailure: false,
    panelFailure: false,
    chunkFailuresRemaining: 0,
    documents: 0,
    apiReads: 0,
    syntheticApiFailures: 0,
    syntheticChunkFailures: 0,
    blockedMutations: 0,
    blockedProviders: 0,
    unexpectedErrors: 0,
    interruptedEvidenceReads: 0,
    artifactMismatches: 0,
    domContentLoads: 0,
    loadEvents: 0,
    mainFrameCommits: 0,
    publicAssetEvents: [],
    failedPublicAssets: [],
  };
  const startedAt = Date.now();
  page.on("domcontentloaded", () => {
    state.domContentLoads++;
  });
  page.on("load", () => {
    state.loadEvents++;
  });
  page.on("framenavigated", (frame) => {
    if (frame === page.mainFrame()) state.mainFrameCommits++;
  });
  page.on("requestfailed", (request) => {
    const url = new URL(request.url());
    if (
      url.origin === origin &&
      /^\/assets\/[^/]+\.js$/.test(url.pathname) &&
      state.failedPublicAssets.length < 32
    )
      state.failedPublicAssets.push({
        path: url.pathname,
        failure: request.failure()?.errorText ?? "unknown",
        elapsed_ms: Date.now() - startedAt,
      });
  });
  const factory = new Set();
  const actualArtifacts = new Map();
  const pending = [];
  const observe = (task) =>
    pending.push(
      task.then(
        () => ({ ok: true }),
        () => {
          state.interruptedEvidenceReads++;
          return { ok: false };
        },
      ),
    );
  page.on("pageerror", (error) => {
    if (
      !/dynamically imported module|module script|chunkloaderror/i.test(
        error.message,
      )
    )
      state.unexpectedErrors++;
  });
  await context.route("**/*", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!["GET", "HEAD", "OPTIONS"].includes(request.method())) {
      state.blockedMutations++;
      return route.abort("blockedbyclient");
    }
    if (request.resourceType() === "document" && url.origin === origin) {
      state.documents++;
      if (state.documents > LIMIT_DOCUMENTS_PER_CASE)
        return route.abort("blockedbyclient");
    }
    if (
      ["fetch", "xhr"].includes(request.resourceType()) &&
      (![origin, indexerOrigin].includes(url.origin) ||
        /\/cosmwasm\/|\/cosmos\/|\/lcd\/|\/rpc\//.test(url.pathname))
    ) {
      state.blockedProviders++;
      return route.abort("blockedbyclient");
    }
    if (
      url.origin === origin &&
      url.pathname === tradeAsset &&
      state.chunkFailuresRemaining > 0
    ) {
      state.chunkFailuresRemaining--;
      state.syntheticChunkFailures++;
      return route.fulfill({
        status: 404,
        contentType: "text/javascript",
        body: "// Synthetic stale Trade chunk for declared QA scope.",
      });
    }
    if (url.origin === indexerOrigin && url.pathname.startsWith("/api/")) {
      state.apiReads++;
      totalApiReads++;
      if (totalApiReads > LIMIT_API_READS)
        return route.abort("blockedbyclient");
      if (
        (state.catalogFailure && url.pathname === "/api/v1/pairs") ||
        (state.panelFailure &&
          /\/pairs\/[^/]+\/(?:candles|trades|limit-book)/.test(url.pathname))
      ) {
        state.syntheticApiFailures++;
        return route.fulfill({
          status: 503,
          contentType: "application/json",
          headers: { "Access-Control-Allow-Origin": "*" },
          body: JSON.stringify({ error: "unavailable" }),
        });
      }
    }
    return route.continue();
  });
  page.on("response", (response) => {
    const url = new URL(response.url());
    if (
      url.origin === origin &&
      expectedAssets.has(url.pathname) &&
      state.publicAssetEvents.length < 64
    )
      state.publicAssetEvents.push({
        path: url.pathname,
        stage: "response_headers",
        status: response.status(),
        elapsed_ms: Date.now() - startedAt,
      });
    if (!response.ok()) return;
    if (url.origin === origin && /^\/assets\/[^/]+\.js$/.test(url.pathname)) {
      observe(
        response.body().then((body) => {
          const sha256 = createHash("sha256").update(body).digest("hex");
          actualArtifacts.set(url.pathname, sha256);
          if (
            expectedAssets.has(url.pathname) &&
            state.publicAssetEvents.length < 64
          )
            state.publicAssetEvents.push({
              path: url.pathname,
              stage: "body_complete",
              elapsed_ms: Date.now() - startedAt,
            });
          if (
            expectedAssets.has(url.pathname) &&
            expectedAssets.get(url.pathname) !== sha256
          )
            state.artifactMismatches++;
          receivedAssets.set(url.pathname, {
            path: url.pathname,
            sha256,
            bytes: body.length,
          });
        }),
      );
    }
    if (url.origin === indexerOrigin && url.pathname === "/api/v1/pairs") {
      observe(
        response.json().then((body) => {
          for (const p of Array.isArray(body.items)
            ? body.items.slice(0, 1000)
            : []) {
            if (factory.size >= 4096) break;
            if (
              typeof p.pair_address === "string" &&
              /^terra1[a-z0-9]{38,64}$/.test(p.pair_address)
            )
              factory.add(p.pair_address);
          }
        }),
      );
    }
  });
  return { context, page, state, factory, pending, actualArtifacts };
}
async function run(browser, width, name, action) {
  process.stderr.write(`recovery ${width}: ${name}\n`);
  const fixture = await setup(browser, width);
  let result = { width, name, verdict: "FAIL" };
  try {
    const findings = await action(fixture);
    await Promise.allSettled(fixture.pending);
    // A deliberately interrupted document can cancel optional preload body
    // reads. The recovered route still requires complete, matching root+Trade
    // bytes for this case; no cancelled response can establish provenance.
    expect(
      [...expectedAssets].every(
        ([p, hash]) => fixture.actualArtifacts.get(p) === hash,
      ),
      "Recovered root and Trade bytes match attested assets",
    ).toBe(true);
    expect(fixture.state.artifactMismatches).toBe(0);
    expect(fixture.state.blockedMutations).toBe(0);
    expect(fixture.state.blockedProviders).toBe(0);
    expect(fixture.state.unexpectedErrors).toBe(0);
    expect(fixture.state.documents).toBeLessThanOrEqual(
      LIMIT_DOCUMENTS_PER_CASE,
    );
    result = { ...result, verdict: "PASS", ...findings };
  } catch (error) {
    process.stderr.write(
      `recovery ${width}: ${name} assertion ${String(error.message)
        .replace(/https?:\/\/\S+/g, "[resource]")
        .slice(0, 2000)}\n`,
    );
    result.failure_class = error.name;
    result.state = await fixture.page
      .evaluate(() => ({
        pathname: location.pathname,
        heading:
          document
            .querySelector(
              '[data-testid="route-error-boundary"] h2, [data-testid="trade-page-heading"]',
            )
            ?.textContent?.trim() ?? null,
        offline: !navigator.onLine,
        visibility: document.visibilityState,
        document_ready_state: document.readyState,
        bootstrap_shell: !!document.querySelector("#trade-bootstrap-shell"),
        actual_trade_heading: !!document.querySelector(
          '[data-testid="trade-page-heading"]',
        ),
        advanced_tabs: !!document.querySelector(
          '[data-testid="trade-order-tab-market"]',
        ),
        risk_checkbox_present: !!document.querySelector(
          'input[type="checkbox"]',
        ),
        gating_dialog_visible: [
          ...document.querySelectorAll('[role="dialog"]'),
        ].some((el) => {
          const rect = el.getBoundingClientRect();
          const style = getComputedStyle(el);
          return (
            rect.width > 0 &&
            rect.height > 0 &&
            style.visibility !== "hidden" &&
            style.display !== "none"
          );
        }),
        workspace_skeleton: !!document.querySelector(
          '[data-testid="trade-workspace-skeleton"]',
        ),
      }))
      .catch(() => null);
  } finally {
    result.counters = fixture.state;
    receipt.trials.push(result);
    process.stderr.write(`recovery ${width}: ${name} ${result.verdict}\n`);
    await fixture.context.close();
  }
}
async function main() {
  const browser = await chromium.launch({
    headless: true,
    timeout: 30000,
    args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
  });
  receipt.browser_version = browser.version();
  const deadline = setTimeout(() => {
    browser.close().catch(() => {});
  }, 300000);
  try {
    for (const width of [1440, 390]) {
      await run(
        browser,
        width,
        "synthetic-catalog-outage-manual-retry",
        async ({ page, state, factory }) => {
          state.catalogFailure = true;
          await page.goto(origin + "/trade", {
            waitUntil: "commit",
            timeout: 30000,
          });
          await expect(page.getByTestId("trade-page-heading")).toBeVisible({
            timeout: 30000,
          });
          await acknowledge(page);
          await expect(
            page.getByRole("button", { name: "Retry", exact: true }),
          ).toBeVisible({ timeout: 20000 });
          expect(new URL(page.url()).pathname).toBe("/trade");
          expect(state.documents).toBe(1);
          state.catalogFailure = false;
          await page
            .getByRole("button", { name: "Retry", exact: true })
            .click();
          return { pathname: await ready(page, factory) };
        },
      );
      await run(
        browser,
        width,
        "synthetic-panels-outage-online-reconnect",
        async ({ context, page, state, factory }) => {
          state.panelFailure = true;
          await page.goto(`${origin}/trade/${pair}`, {
            waitUntil: "commit",
            timeout: 30000,
          });
          await ready(page, factory);
          await expect(
            page.getByTestId("trade-indexer-outage-banner"),
          ).toBeVisible({ timeout: 20000 });
          await expect(page.getByTestId("trade-chart-unavailable")).toBeVisible(
            { timeout: 20000 },
          );
          await expect(
            page.getByTestId("trade-book-unavailable-bid"),
          ).toBeVisible({ timeout: 20000 });
          await expect(
            page.getByTestId("trade-book-unavailable-ask"),
          ).toBeVisible({ timeout: 20000 });
          await page
            .getByTestId(
              width >= 1024
                ? "trade-desktop-tape-toggle"
                : "trade-sub-lg-tape-disclosure-toggle",
            )
            .click();
          await expect(page.getByTestId("trade-tape-unavailable")).toBeVisible({
            timeout: 20000,
          });
          expect(state.documents).toBe(1);
          state.panelFailure = false;
          await context.setOffline(true);
          await context.setOffline(false);
          await expect(
            page.getByTestId("trade-indexer-outage-banner"),
          ).toHaveCount(0, { timeout: 20000 });
          for (const id of [
            "trade-chart-unavailable",
            "trade-book-unavailable-bid",
            "trade-book-unavailable-ask",
            "trade-tape-unavailable",
          ])
            await expect(page.getByTestId(id)).toHaveCount(0, {
              timeout: 20000,
            });
          expect(new URL(page.url()).pathname).toBe(`/trade/${pair}`);
          return { pathname: await ready(page, factory) };
        },
      );
      await run(
        browser,
        width,
        "offline-lazy-import-manual-retry-preserves-trade",
        async ({ context, page, state, factory }) => {
          await page.goto(origin + "/", {
            waitUntil: "commit",
            timeout: 30000,
          });
          await expect(page.getByTestId("swap-you-pay-amount")).toBeVisible({
            timeout: 30000,
          });
          await acknowledge(page);
          await context.setOffline(true);
          expect(await page.evaluate(() => navigator.onLine)).toBe(false);
          await page
            .locator(
              width < 768 ? "nav.app-mobile-nav-shell" : "nav.app-desktop-nav",
            )
            .getByRole("link", { name: "Trade", exact: true })
            .click();
          await expect(page.getByTestId("route-error-boundary")).toContainText(
            "Page unavailable",
          );
          expect(new URL(page.url()).pathname).toBe("/trade");
          expect(state.documents).toBe(1);
          await context.setOffline(false);
          await page
            .getByRole("button", { name: "Try Again", exact: true })
            .click();
          const pathname = await ready(page, factory);
          expect(state.documents).toBeLessThanOrEqual(2);
          return { pathname };
        },
      );
      await run(
        browser,
        width,
        "single-stale-chunk-404-one-shot-reload",
        async ({ page, state, factory }) => {
          state.chunkFailuresRemaining = 1;
          await page.goto(origin + "/trade", {
            waitUntil: "commit",
            timeout: 30000,
          });
          const pathname = await ready(page, factory);
          expect(state.syntheticChunkFailures).toBe(1);
          expect(state.documents).toBe(2);
          expect(
            await page.evaluate(() =>
              sessionStorage.getItem("cl8y-dex-stale-chunk-reload"),
            ),
          ).toBeNull();
          return { pathname };
        },
      );
      await run(
        browser,
        width,
        "repeated-stale-chunk-404-bounded-error-and-explicit-reload",
        async ({ page, state, factory }) => {
          state.chunkFailuresRemaining = 10;
          await page.goto(origin + "/trade", {
            waitUntil: "commit",
            timeout: 30000,
          });
          await expect(page.getByTestId("route-error-boundary")).toContainText(
            "Page unavailable",
            { timeout: 30000 },
          );
          await acknowledge(page);
          expect(new URL(page.url()).pathname).toBe("/trade");
          expect(state.documents).toBe(2);
          expect(state.syntheticChunkFailures).toBe(2);
          await page
            .getByRole("button", { name: "Try Again", exact: true })
            .click();
          await expect(page.getByTestId("route-error-boundary")).toContainText(
            "Page unavailable",
          );
          await page.waitForTimeout(300);
          expect(state.documents).toBe(2);
          state.chunkFailuresRemaining = 0;
          await page.getByTestId("route-error-reload-app").click();
          const pathname = await ready(page, factory);
          expect(state.documents).toBe(3);
          return { pathname };
        },
      );
    }
  } finally {
    clearTimeout(deadline);
    await browser.close();
    receipt.completed_at_utc = new Date().toISOString();
    receipt.total_api_reads = totalApiReads;
    receipt.served_assets = [...receivedAssets.values()].sort((a, b) =>
      a.path.localeCompare(b.path),
    );
    receipt.verdict =
      receipt.trials.length === 10 &&
      receipt.trials.every((t) => t.verdict === "PASS") &&
      [...expectedAssets].every(
        ([p, hash]) => receivedAssets.get(p)?.sha256 === hash,
      )
        ? "PASS"
        : "FAIL";
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + "\n", {
      mode: 0o600,
    });
    process.stdout.write(
      `${receipt.verdict}: ${receipt.trials.filter((t) => t.verdict === "PASS").length}/${receipt.trials.length} declared synthetic Trade recovery workflows; receipt ${output}\n`,
    );
    if (receipt.verdict !== "PASS") process.exitCode = 1;
  }
}
main().catch(() => {
  process.stderr.write(
    "FAIL: bounded Trade recovery helper could not complete.\n",
  );
  process.exitCode = 1;
});
