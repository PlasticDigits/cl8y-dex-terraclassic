#!/usr/bin/env node
// Bounded anonymous deployed Swap/Trade explanation checks. No signing or screenshots.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const { createRequire } = require("node:module");
const ROOT = path.resolve(__dirname, "../..");
const frontend = createRequire(path.join(ROOT, "frontend-dapp/package.json"));
const FRONTEND = "https://dex.cl8y.com";
const BACKEND = "https://indexer.dex.cl8y.com";
const COPIES = [
  "Wrapping costs",
  "Trading costs",
  "Limited liquidity",
  "Pool and order pricing",
  "Better pool or order pricing",
  "Different pricing",
  "Combined costs and pricing",
  "Matches reference prices",
];

function assert(condition, code) {
  if (!condition) throw new Error(code);
}
function nodesOf(node, output = []) {
  if (!node || typeof node !== "object") return output;
  if (typeof node.type === "string") output.push(node);
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) value.forEach((child) => nodesOf(child, output));
    else if (value && typeof value === "object") nodesOf(value, output);
  }
  return output;
}

/** Reuse the actual strict validator and original-input/two-frame closure, as its existing tests do. */
function sharedSources() {
  const source = fs.readFileSync(
    path.join(__dirname, "browser-live-trading-quotes-1473.cjs"),
    "utf8",
  );
  assert(source.length < 128 * 1024, "SharedSourceBudget");
  const nodes = nodesOf(frontend("@babel/parser").parse(source));
  const text = (node) => source.slice(node.start, node.end);
  const init = nodes.find(
    (node) =>
      node.type === "ArrowFunctionExpression" &&
      text(node).includes("function validate(body, params, now)"),
  );
  const declarations = init?.body.body.filter(
    (node) =>
      node.type === "VariableDeclaration" &&
      node.declarations.some((row) =>
        ["S", "integer", "decimal"].includes(row.id?.name),
      ),
  );
  const validate = nodesOf(init).find(
    (node) =>
      node.type === "FunctionDeclaration" && node.id?.name === "validate",
  );
  const measure = nodes.find(
    (node) =>
      node.type === "ArrowFunctionExpression" &&
      node.params[0]?.name === "expected" &&
      text(node).includes("window.__measurement1473"),
  );
  assert(
    declarations?.length === 3 && validate && measure,
    "SharedClosureShape",
  );
  const edits = [];
  for (const node of nodesOf(measure)) {
    if (
      node.type === "CallExpression" &&
      node.callee.type === "MemberExpression" &&
      node.callee.object.name === "document" &&
      node.callee.property.name === "querySelector"
    ) {
      edits.push([node.callee.start, node.callee.end, "window.__query1483"]);
    }
    if (
      node.type === "VariableDeclarator" &&
      node.id.name === "value" &&
      text(node.init).includes("output.value")
    ) {
      edits.push([
        node.init.start,
        node.init.end,
        "window.__receive1483(output, expected.entry)",
      ]);
    }
    if (
      node.type === "ReturnStatement" &&
      node.argument?.type === "ObjectExpression" &&
      node.argument.properties.some(
        (p) => p.key?.name === "kind" && p.value?.value === "useful",
      )
    ) {
      edits.push([
        node.argument.start,
        node.argument.end,
        `(window.__explanationReady1483(expected, latest) ? ${text(node.argument)} : null)`,
      ]);
    }
  }
  assert(
    edits.some(([, , replacement]) =>
      replacement.includes("__explanationReady1483"),
    ),
    "SharedReadinessShape",
  );
  let measurement = text(measure);
  for (const [start, end, replacement] of edits.sort((a, b) => b[0] - a[0])) {
    measurement =
      measurement.slice(0, start - measure.start) +
      replacement +
      measurement.slice(end - measure.start);
  }
  const typescript = frontend("typescript");
  const explanation = typescript.transpileModule(
    fs.readFileSync(
      path.join(ROOT, "frontend-dapp/src/utils/routeEconomicExplanation.ts"),
      "utf8",
    ),
    {
      compilerOptions: {
        target: typescript.ScriptTarget.ES2022,
        module: typescript.ModuleKind.CommonJS,
      },
    },
  ).outputText;
  assert(!/require\(/.test(explanation), "ExplanationDependencyShape");
  const precision = typescript.transpileModule(
    fs.readFileSync(
      path.join(ROOT, "frontend-dapp/src/utils/swapRouteSlippage.ts"),
      "utf8",
    ),
    {
      compilerOptions: {
        target: typescript.ScriptTarget.ES2022,
        module: typescript.ModuleKind.CommonJS,
      },
    },
  ).outputText;
  assert(!/require\(/.test(precision), "PrecisionDependencyShape");
  return {
    precision: `(() => { const exports = {}; ${precision}; return exports.preciseFairDeviationPercent; })()`,
    format: `(() => { const exports = {}; ${precision}; return exports.formatTradingPriceDifference; })()`,
    validator: `(() => { ${declarations.map(text).join("\n")} return ${text(validate)}; })()`,
    measurement,
    explanation: `(() => { const exports = {}; ${explanation}; return exports.routeEconomicExplanation; })()`,
  };
}

function checkAttestation(attested, sha) {
  assert(/^[0-9a-f]{40}$/.test(sha || ""), "RevisionShape");
  assert(
    attested.overall === "PASS" &&
      attested.expected_revision === sha &&
      attested.frontend?.source_sha === sha &&
      attested.backend?.git_sha === sha &&
      attested.backend_deployment?.source_sha === sha &&
      attested.frontend?.deployment_status === "finished" &&
      attested.backend_deployment?.deployment_status === "finished",
    "IndependentAttestation",
  );
  const artifact = attested.frontend.artifact;
  assert(
    /^[0-9a-f]{64}$/.test(artifact?.html_sha256 || "") &&
      Array.isArray(artifact.root_scripts) &&
      artifact.root_scripts.length > 0 &&
      artifact.root_scripts.length <= 8 &&
      artifact.root_scripts.every(
        (row) =>
          /^[-A-Za-z0-9_.]+\.js$/.test(row.asset || "") &&
          /^[0-9a-f]{64}$/.test(row.sha256 || ""),
      ),
    "ArtifactShape",
  );
  return artifact;
}

function installRuntime(config) {
  const visible = (node) => {
    const r = node?.getBoundingClientRect(),
      s = node ? getComputedStyle(node) : null;
    if (
      !(
        r?.width > 0 &&
        r.height > 0 &&
        r.left >= 0 &&
        r.top >= 0 &&
        r.right <= innerWidth &&
        r.bottom <= innerHeight &&
        s?.display !== "none" &&
        s?.visibility !== "hidden" &&
        Number(s?.opacity) > 0
      )
    )
      return false;
    for (
      let parent = node.parentElement;
      parent;
      parent = parent.parentElement
    ) {
      const style = getComputedStyle(parent),
        rect = parent.getBoundingClientRect();
      if (
        /(auto|scroll|hidden|clip)/.test(style.overflowY) &&
        (r.top < rect.top || r.bottom > rect.bottom)
      )
        return false;
      if (
        /(auto|scroll|hidden|clip)/.test(style.overflowX) &&
        (r.left < rect.left || r.right > rect.right)
      )
        return false;
    }
    return true;
  };
  window.__query1483 = (selector) =>
    [...document.querySelectorAll(selector)].find(visible) ||
    document.querySelector(selector);
  window.__receive1483 = (output, entry) =>
    entry === "trade"
      ? (output.textContent || "")
          .replaceAll(",", "")
          .trim()
          .match(/^\d+(?:\.\d+)?/)?.[0] || ""
      : output.value?.replaceAll(",", "") || "";
  window.__quotes1473 = [];
  window.__explanationReady1483 = (expected, latest) => {
    const diag = latest.diagnostic;
    const pay = window.__query1483(
      `[data-testid="${expected.entry === "trade" ? "limit-order-escrow-amount-input" : "swap-you-pay-amount"}"]`,
    );
    const scope =
      expected.entry === "trade"
        ? pay.closest('[role="tabpanel"]') || document
        : document;
    const prefix = expected.entry === "trade" ? "trade-market" : "swap";
    const receive = window.__query1483(
      `[data-testid="${expected.entry === "trade" ? "trade-market-expected-receive" : "swap-you-receive"}"]`,
    );
    if (!diag?.valid || !visible(receive)) return false;
    const low = diag.precise_percent <= 5;
    const outside = scope.querySelector(
      `[data-testid="${prefix}-impact-aside"]`,
    );
    const node = low
      ? scope.querySelector(`[data-testid="${prefix}-price-details"]`)
      : outside;
    if (!node || (!low && !visible(node))) return false;
    if (
      low &&
      (outside ||
        !node.closest(
          `[data-testid="${expected.entry === "trade" ? "trade-market-signing-details" : "swap-pre-submit-details"}"]`,
        ))
    )
      return false;
    const cause =
      diag.explanation === "Matches reference prices"
        ? ""
        : ` — ${diag.explanation.toLowerCase()}`;
    const text = node.textContent.replace(/\s+/g, " ").trim();
    const expectedText = `Price difference ${diag.percent}%${cause}.`;
    const alerts = [...scope.querySelectorAll('[role="alert"]')].filter(
      visible,
    );
    return (
      text === expectedText &&
      (text.match(/%/g) || []).length === 1 &&
      alerts.length === (low ? 0 : 1) &&
      ![
        ...scope.querySelectorAll(
          '[data-testid="swap-expected-slippage"],[data-testid="trade-market-quote-extras"]',
        ),
      ]
        .filter(visible)
        .some((node) =>
          /(?:Hop spread|Worst hop spread|Price impact|Expected slippage|Fair-reference)\s*[:≈]?\s*\d/i.test(
            node.textContent || "",
          ),
        )
    );
  };
  localStorage.setItem("cl8y-dex-number-display-mode", "full");
  const original = window.fetch.bind(window);
  window.fetch = async (...args) => {
    const input = args[0],
      url = new URL(
        typeof input === "string" ? input : input?.url || String(input),
        location.href,
      );
    const method = String(
      args[1]?.method || input?.method || "GET",
    ).toUpperCase();
    if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
      await window.__prohibited1483("write");
      throw new Error("ReadOnly");
    }
    const quote =
      method === "GET" &&
      ["/api/v1/route/solve", "/api/v1/route/solve/best"].includes(
        url.pathname,
      );
    const params = quote ? Object.fromEntries(url.searchParams) : null;
    if (quote && (params.trader != null || params.sender != null)) {
      await window.__prohibited1483("actor");
      throw new Error("AnonymousOnly");
    }
    const row = quote
      ? { started: performance.now(), status: null, params }
      : null;
    if (row) {
      window.__quotes1473.push(row);
      if (window.__quotes1473.length > 100) window.__quotes1473.shift();
    }
    const response = await original(...args);
    if (row) {
      row.status = response.status;
      row.headers_received = performance.now();
      void (async () => {
        try {
          const reader = response.clone().body.getReader(),
            chunks = [],
            limit = response.status === 200 ? 2 * 1024 * 1024 : 4096;
          let size = 0;
          while (true) {
            const next = await reader.read();
            if (next.done) break;
            size += next.value.length;
            if (size > limit) {
              await reader.cancel();
              throw new Error("BodyBudget");
            }
            chunks.push(next.value);
          }
          const bytes = new Uint8Array(size);
          let offset = 0;
          for (const chunk of chunks) {
            bytes.set(chunk, offset);
            offset += chunk.length;
          }
          const body = JSON.parse(new TextDecoder().decode(bytes));
          row.classified =
            response.status === 200 &&
            window.__validate1483(body, params, Date.now());
          row.useful = row.classified;
          if (row.useful) {
            row.output = body.estimated_amount_out_net;
            row.minimum = body.execution_metadata.minimum_receive;
            row.observed = body.execution_metadata.observed_at_ms;
            row.expires = body.execution_metadata.expires_at_ms;
            row.generation = body.execution_metadata.generation;
            const credit = body.execution_metadata.wrapper?.route_input_amount;
            if (
              body.execution_metadata.wrapper?.wrap_input === true &&
              typeof credit === "string" &&
              /^[1-9]\d{0,38}$/.test(credit) &&
              BigInt(credit) <= BigInt(params.amount_in)
            )
              row.native_credit = credit;
            const explanation = window.__explain1483(body),
              fair = BigInt(body.spot_amount_out),
              net = BigInt(row.output);
            row.diagnostic = {
              explanation,
              precise_percent: Number(
                window.__precise1483(String(net), String(fair)),
              ),
              percent: window.__format1483(
                Number(window.__precise1483(String(net), String(fair))),
              ),
              valid:
                body.economic_attribution?.status === "available" &&
                config.copies.includes(explanation),
            };
          }
        } catch {
          row.classified = false;
          row.useful = false;
        }
        row.ended = performance.now();
      })().catch(() => {});
    }
    return response;
  };
}

function measurementSource(shared, entry) {
  let source = shared.measurement;
  if (entry === "trade")
    source = source
      .replaceAll("swap-you-pay-amount", "limit-order-escrow-amount-input")
      .replaceAll("swap-you-receive", "trade-market-expected-receive")
      .replaceAll("swap-price-data-loading", "trade-market-price-data-loading")
      .replaceAll("swap-quote-error", "trade-market-quote-error");
  return source;
}

function browserSampleFailed(row) {
  return (
    !row.useful ||
    !row.classified ||
    !row.policy_opt_in ||
    row.secondary_estimate_present ||
    !Number.isFinite(row.input_to_dom_ms) ||
    row.input_to_dom_ms < 0 ||
    row.input_to_dom_ms > 750
  );
}

async function run() {
  const SHA = process.env.EXPECTED_SOURCE_SHA,
    file = process.env.DEX1483_ATTESTATION_FILE;
  assert(file && fs.statSync(file).size <= 64 * 1024, "AttestationRequired");
  const artifact = checkAttestation(
    JSON.parse(fs.readFileSync(file, "utf8")),
    SHA,
  );
  for (const file of [
    "frontend-dapp/src/utils/routeEconomicExplanation.ts",
    "frontend-dapp/src/utils/swapRouteSlippage.ts",
    "scripts/qa/browser-live-trading-quotes-1473.cjs",
  ]) {
    const committed = execFileSync("git", ["show", SHA + ":" + file], {
      cwd: ROOT,
      maxBuffer: 256 * 1024,
      stdio: ["ignore", "pipe", "ignore"],
    });
    assert(
      committed.equals(fs.readFileSync(path.join(ROOT, file))),
      "LocalValidatorRevision",
    );
  }
  const shared = sharedSources(),
    output =
      process.env.DEX1483_BROWSER_OUTPUT ||
      "/tmp/dex1483-browser-explanation.json";
  const started = Date.now(),
    elapsedLimit = 180000;
  let ownedBrowser,
    apiReads = 0,
    quoteReads = 0,
    blockedWrites = 0,
    blockedActors = 0,
    providerReads = 0,
    blockedWebSockets = 0,
    measured = false;
  const receipt = {
    issue: 1483,
    revision: SHA,
    utc_start: new Date().toISOString(),
    browser_budget_ms: 750,
    socket_budget_ms: 500,
    api_read_limit: 100,
    quote_read_limit: 100,
    elapsed_limit_ms: elapsedLimit,
    read_only: true,
    signing: false,
    samples: [],
    completed: false,
    artifacts: [],
    skips: [
      "Physical device and wallet signing",
      "Settlement and independent contract reserves",
      "Sequential controls use actual previously quoted native credit; no same-cohort or equal-net claim",
    ],
  };
  const save = () => {
    receipt.api_reads = apiReads;
    receipt.quote_reads = quoteReads;
    receipt.prevented_writes = blockedWrites;
    receipt.actor_requests = blockedActors;
    receipt.measured_provider_requests = providerReads;
    receipt.blocked_websockets = blockedWebSockets;
    receipt.overall =
      receipt.failure_class ||
      blockedWrites ||
      blockedActors ||
      providerReads ||
      blockedWebSockets ||
      receipt.samples.some((row) => row.failed)
        ? "FAIL"
        : receipt.completed
          ? "PASS WITH SKIPS"
          : "RUNNING";
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + "\n", {
      mode: 0o600,
    });
  };
  const bounded = async (url, limit) => {
    assert(Date.now() - started < elapsedLimit && apiReads < 100, "ReadBudget");
    apiReads++;
    const response = await fetch(url, {
      signal: AbortSignal.timeout(
        Math.min(10000, elapsedLimit - (Date.now() - started)),
      ),
      redirect: "error",
    });
    assert(response.ok, "PublicRead");
    const reader = response.body.getReader(),
      chunks = [];
    let size = 0;
    while (true) {
      const next = await reader.read();
      if (next.done) break;
      size += next.value.length;
      if (size > limit) {
        await reader.cancel();
        throw new Error("BodyBudget");
      }
      chunks.push(Buffer.from(next.value));
    }
    return Buffer.concat(chunks);
  };
  const attest = async () => {
    const health = JSON.parse(await bounded(BACKEND + "/health", 4096));
    assert(health.git_sha === SHA, "BackendRevision");
    const html = await bounded(FRONTEND + "/", 128 * 1024);
    const scripts = [
      ...html.toString().matchAll(/<script\b[^>]*\bsrc=["']([^"']+)["']/g),
    ].map((row) => row[1]);
    assert(
      scripts.length > 0 &&
        scripts.length <= 8 &&
        scripts.every((asset) => /^\/assets\/[-A-Za-z0-9_.]+\.js$/.test(asset)),
      "RootScripts",
    );
    const roots = [];
    for (const asset of scripts)
      roots.push({
        asset: path.basename(asset),
        sha256: crypto
          .createHash("sha256")
          .update(await bounded(FRONTEND + asset, 8 * 1024 * 1024))
          .digest("hex"),
      });
    const actual = {
      html_sha256: crypto.createHash("sha256").update(html).digest("hex"),
      root_scripts: roots,
    };
    assert(
      actual.html_sha256 === artifact.html_sha256 &&
        JSON.stringify(roots) ===
          JSON.stringify(
            artifact.root_scripts.map(({ asset, sha256 }) => ({
              asset,
              sha256,
            })),
          ),
      "ServedArtifact",
    );
    receipt.artifacts.push(actual);
  };
  const timer = setTimeout(() => {
    receipt.failure_class = "ElapsedBudget";
    void ownedBrowser?.close().catch(() => {});
  }, elapsedLimit);
  timer.unref();
  try {
    await attest();
    const assets = {
      ...JSON.parse(
        fs.readFileSync(path.join(ROOT, "docs/qa/issue-1473/matrix.json")),
      ).assets,
      ...JSON.parse(
        fs.readFileSync(path.join(ROOT, "docs/qa/issue-1479/matrix.json")),
      ).assets,
    };
    const catalog = JSON.parse(
      await bounded(
        BACKEND + "/api/v1/tokens/" + assets.cLUNC.identity + "/pairs",
        2 * 1024 * 1024,
      ),
    );
    assert(Array.isArray(catalog) && catalog.length <= 256, "CatalogBound");
    const pairs = catalog.filter(
      (pair) =>
        pair.is_active !== false &&
        pair.asset_0 &&
        pair.asset_1 &&
        [pair.asset_0.contract_addr, pair.asset_1.contract_addr].includes(
          assets.cLUNC.identity,
        ) &&
        [pair.asset_0.contract_addr, pair.asset_1.contract_addr].includes(
          assets.UST1.identity,
        ) &&
        /^terra1[a-z0-9]{38,80}$/.test(pair.pair_address || ""),
    );
    assert(pairs.length === 1, "GenuineTradePair");
    receipt.trade_factory_pair = pairs[0].pair_address;
    const { chromium } = frontend("@playwright/test");
    ownedBrowser = await chromium.launch({
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
    for (const [view, viewport] of [
      ["desktop", { width: 1440, height: 1000 }],
      ["phone", { width: 390, height: 844 }],
    ]) {
      const context = await ownedBrowser.newContext({
        viewport,
        serviceWorkers: "block",
      });
      await context.exposeBinding("__prohibited1483", (_, kind) => {
        if (kind === "actor") blockedActors++;
        else blockedWrites++;
      });
      await context.routeWebSocket("**/*", (socket) => {
        blockedWebSockets++;
        socket.close();
      });
      await context.addInitScript({
        content: `window.__validate1483 = ${shared.validator}; window.__explain1483 = ${shared.explanation}; window.__precise1483 = ${shared.precision}; window.__format1483 = ${shared.format};`,
      });
      await context.addInitScript(installRuntime, { copies: COPIES });
      await context.route("**/*", async (route) => {
        const request = route.request(),
          url = new URL(request.url()),
          method = request.method();
        if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
          blockedWrites++;
          return route.abort("blockedbyclient");
        }
        const api = url.origin === BACKEND && url.pathname.startsWith("/api/");
        if (api) {
          apiReads++;
          if (apiReads > 100) {
            receipt.failure_class = "ReadBudget";
            return route.abort("blockedbyclient");
          }
        }
        if (
          ["/api/v1/route/solve", "/api/v1/route/solve/best"].includes(
            url.pathname,
          )
        ) {
          quoteReads++;
          if (quoteReads > 100) return route.abort("blockedbyclient");
        }
        if (url.searchParams.has("trader") || url.searchParams.has("sender")) {
          blockedActors++;
          return route.abort("blockedbyclient");
        }
        if (
          measured &&
          ((["fetch", "xhr"].includes(request.resourceType()) &&
            ![FRONTEND, BACKEND].includes(url.origin)) ||
            /\/(cosmos|cosmwasm|terra|wasm|rpc|lcd|abci_query)(\/|$)/.test(
              url.pathname,
            ))
        ) {
          providerReads++;
          return route.abort("blockedbyclient");
        }
        if (Date.now() - started >= elapsedLimit)
          return route.abort("blockedbyclient");
        return route.continue();
      });
      const page = await context.newPage();
      const nativeCredits = new Map();
      for (const entry of ["swap_native", "swap_credit", "trade"]) {
        const trade = entry === "trade",
          input = entry === "swap_native" ? assets.LUNC : assets.cLUNC,
          target = trade ? assets.UST1 : assets.USTR;
        await page.goto(
          FRONTEND +
            (trade
              ? "/trade/" + pairs[0].pair_address
              : "/?" +
                new URLSearchParams({
                  from: input.identity,
                  to: target.identity,
                })),
          { waitUntil: "domcontentloaded", timeout: 30000 },
        );
        const risk = page.getByRole("checkbox", {
          name: /I understand these risks/,
        });
        if (await risk.isVisible().catch(() => false)) {
          await risk.check();
          await page
            .getByRole("button", { name: "Continue to the app" })
            .click();
        }
        if (trade)
          await page
            .locator('[data-testid="trade-order-tab-market"]:visible')
            .first()
            .click();
        const payId = trade
            ? "limit-order-escrow-amount-input"
            : "swap-you-pay-amount",
          receiveId = trade
            ? "trade-market-expected-receive"
            : "swap-you-receive";
        const pay = page.locator(`[data-testid="${payId}"]:visible`).first();
        await pay.waitFor({ timeout: 20000 });
        if (trade) {
          await page
            .locator('[data-testid="trade-market-advanced-toggle"]:visible')
            .first()
            .click();
          await page
            .locator('[data-testid="trade-market-slippage-preset-5"]:visible')
            .first()
            .click();
          await page
            .locator('[data-testid="trade-market-advanced-toggle"]:visible')
            .first()
            .click();
        } else {
          await page
            .getByRole("button", { name: "Settings", exact: true })
            .click();
          await page.getByTestId("swap-slippage-preset-5").click();
          await page
            .getByRole("button", { name: "Settings", exact: true })
            .click();
        }
        await pay.fill("0.1");
        if (trade) {
          await page.waitForFunction(
            () =>
              window.__quotes1473.some(
                (row) => row.useful && row.params.amount_in === "100000",
              ),
            undefined,
            { timeout: 15000 },
          );
          const wrong = await page.evaluate(
            (input) =>
              window.__quotes1473
                .filter((row) => row.params.amount_in === "100000")
                .at(-1)?.params.token_in !== input,
            input.identity,
          );
          if (wrong) {
            const ask = page
              .locator('[data-testid="trade-ticket-side-ask"]:visible')
              .first();
            const selected = await ask.getAttribute("aria-checked");
            await page
              .locator(
                `[data-testid="trade-ticket-side-${selected === "true" ? "bid" : "ask"}"]:visible`,
              )
              .first()
              .click();
            await pay.fill("0.2");
          }
        }
        await page.waitForFunction(
          () =>
            window.__quotes1473.some(
              (row) =>
                row.useful && row.diagnostic?.valid && row.expires > Date.now(),
            ),
          undefined,
          { timeout: 15000 },
        );
        for (const amount of ["1", "1000", "4000"]) {
          const raw =
            entry === "swap_credit"
              ? nativeCredits.get(amount)
              : String(BigInt(amount) * 1000000n);
          if (!raw) {
            receipt.samples.push({
              viewport: view,
              entry,
              native_human_amount: amount,
              failed: true,
              failure_class: "MatchingCreditUnavailable",
            });
            save();
            continue;
          }
          const human =
            entry === "swap_credit"
              ? (BigInt(raw) / 1000000n).toString() +
                "." +
                (BigInt(raw) % 1000000n).toString().padStart(6, "0")
              : amount;
          await pay.evaluate(
            (node, { receiveId, prefix }) => {
              const find = (id) =>
                [...document.querySelectorAll(`[data-testid="${id}"]`)].find(
                  (n) => n.getBoundingClientRect().height > 0,
                );
              const nodes = [
                node,
                find(receiveId),
                find(prefix + "-impact-aside"),
              ].filter(Boolean);
              const center = () => {
                const rects = nodes.map((n) => n.getBoundingClientRect());
                return (
                  (Math.min(...rects.map((r) => r.top)) +
                    Math.max(...rects.map((r) => r.bottom))) /
                  2
                );
              };
              for (
                let parent = node.parentElement;
                parent;
                parent = parent.parentElement
              ) {
                if (
                  /(auto|scroll)/.test(getComputedStyle(parent).overflowY) &&
                  parent.scrollHeight > parent.clientHeight
                ) {
                  const rect = parent.getBoundingClientRect();
                  parent.scrollBy({
                    top: center() - rect.top - parent.clientHeight / 2,
                    behavior: "instant",
                  });
                  break;
                }
              }
              scrollBy({
                top: center() - innerHeight / 2,
                behavior: "instant",
              });
            },
            { receiveId, prefix: trade ? "trade-market" : "swap" },
          );
          await pay.fill("0");
          await page.waitForTimeout(100);
          const expected = {
            input: input.identity,
            output: target.identity,
            amount: raw,
            spread: "0.05",
            decimals: target.decimals,
            offerDecimals: input.decimals,
            entry: trade ? "trade" : "swap",
          };
          await page.evaluate(
            new Function(
              `return (${measurementSource(shared, expected.entry)})`,
            )(),
            expected,
          );
          measured = true;
          await pay.fill(human);
          await page
            .waitForFunction(
              () => window.__measurement1473?.done != null,
              undefined,
              { timeout: 15000, polling: 10 },
            )
            .catch(() => {});
          measured = false;
          const row = await page.evaluate(() => {
            const result = window.__measurement1473.read(),
              latest = window.__measurement1473.latest;
            return {
              ...result,
              explanation: latest?.diagnostic?.explanation,
              displayed_percent: latest?.diagnostic?.percent,
              detail_case: latest?.diagnostic?.precise_percent <= 5,
              quoted_wrap_credit_raw: latest?.native_credit,
            };
          });
          row.failed = browserSampleFailed(row);
          if (!row.failed && row.detail_case) {
            const details = page
              .locator(
                `[data-testid="${trade ? "trade-market-signing-details" : "swap-pre-submit-details"}"]:visible`,
              )
              .first();
            await details.locator("summary").click();
            const detail = details.locator(
              `[data-testid="${trade ? "trade-market" : "swap"}-price-details"]`,
            );
            row.progressive_disclosure_verified =
              (await detail.isVisible()) &&
              (await detail.textContent()).includes(
                `Price difference ${row.displayed_percent}%`,
              );
            if (!row.progressive_disclosure_verified) row.failed = true;
            await details.locator("summary").click();
          }

          if (
            entry === "swap_native" &&
            row.useful &&
            row.quoted_wrap_credit_raw
          )
            nativeCredits.set(amount, row.quoted_wrap_credit_raw);
          if (!row.failed) delete row.diagnostics;
          receipt.samples.push({
            viewport: view,
            entry,
            human_amount: human,
            raw_input: raw,
            ...row,
          });
          save();
        }
      }
      await context.close();
    }
    await attest();
    assert(receipt.samples.length === 18, "IncompleteCorpus");
    receipt.completed = true;
  } catch (error) {
    receipt.failure_class =
      error.name === "TimeoutError" ? "TimeoutError" : "BoundedCheckFailure";
  } finally {
    measured = false;
    clearTimeout(timer);
    await ownedBrowser?.close().catch(() => {});
    receipt.utc_end = new Date().toISOString();
    save();
  }
  console.log(
    JSON.stringify({
      overall: receipt.overall,
      samples: receipt.samples.length,
      api_reads: apiReads,
      quote_reads: quoteReads,
    }),
  );
  return receipt.overall === "FAIL" ? 1 : 0;
}

module.exports = {
  sharedSources,
  checkAttestation,
  installRuntime,
  measurementSource,
  browserSampleFailed,
  COPIES,
  run,
};
if (require.main === module)
  run()
    .then((code) => {
      process.exitCode = code;
    })
    .catch(() => {
      const failure = {
        issue: 1483,
        overall: "FAIL",
        failure_class: "PrerequisiteValidation",
        completed: false,
        samples: [],
        utc_end: new Date().toISOString(),
        read_only: true,
        signing: false,
      };
      try {
        fs.writeFileSync(
          process.env.DEX1483_BROWSER_OUTPUT ||
            "/tmp/dex1483-browser-explanation.json",
          JSON.stringify(failure, null, 2) + "\n",
          { mode: 0o600 },
        );
      } catch {}
      console.error(
        JSON.stringify({
          overall: failure.overall,
          failure_class: failure.failure_class,
        }),
      );
      process.exitCode = 1;
    });
