#!/usr/bin/env node
// Actual anonymous Swap/Trade and exact progress intent over original expiry cycles.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createRequire } = require('node:module');
const { execFileSync } = require('node:child_process');
const helpers = require('./browser-route-attribution-1483.cjs');
const ROOT = path.resolve(__dirname, '../..');
const frontend = createRequire(path.join(ROOT, 'frontend-dapp/package.json'));
const FRONTEND = 'https://dex.cl8y.com';
const BACKEND = 'https://indexer.dex.cl8y.com';
const { recordThenClearInput, countBoundedRead, recordRequestDiagnostic } = require('./browser-case-lifecycle.cjs');
const MAX_READS = 600;
const assert = (value, code) => { if (!value) throw new Error(code); };

function intent(params) {
  const keys = ['token_in', 'token_out', 'amount_in', 'amount_out', 'max_spread',
    'protection_policy', 'pool_only', 'hybrid_optimize', 'max_maker_fills', 'discount_bps'];
  assert(!params.trader && !params.sender, 'Actor');
  return Object.fromEntries(keys.map(key => [key, params[key] ?? null]));
}

function matchingProgress(progress, quotes) {
  const key = JSON.stringify(intent(progress));
  return quotes.some(row => JSON.stringify(intent(row.params)) === key);
}

function displayRaw(value, decimals) {
  if (typeof value !== 'string' || value.length > 96 || !/^\d+(\.\d+)?$/.test(value) ||
    !Number.isInteger(decimals) || decimals < 0 || decimals > 38) return null;
  const [whole, fraction = ''] = value.split('.');
  if (fraction.length > decimals) return null;
  return (BigInt(whole) * 10n ** BigInt(decimals) + BigInt(fraction.padEnd(decimals, '0') || '0')).toString();
}

function sampleFailed(sample) {
  return helpers.browserSampleFailed(sample) || !sample.progress_intent_matches ||
    !sample.validated_latest_adopted;
}

function observeProgress() {
  window.__progress1489 = [];
  const original = window.fetch.bind(window);
  window.fetch = async (...args) => {
    const input = args[0];
    const url = new URL(typeof input === 'string' ? input : input?.url || String(input), location.href);
    if (url.pathname === '/api/v1/route/solve/progress') {
      window.__progress1489.push(Object.fromEntries(url.searchParams));
      if (window.__progress1489.length > 128) window.__progress1489.shift();
    }
    return original(...args);
  };
}

async function run() {
  const sha = process.env.EXPECTED_SOURCE_SHA;
  const artifact = helpers.checkAttestation(JSON.parse(fs.readFileSync(
    process.env.DEX1489_ATTESTATION_FILE, 'utf8')), sha);
  assert(execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT }).toString().trim() === sha,
    'SourceRevision');
  for (const file of ['frontend-dapp/src/utils/routeEconomicExplanation.ts',
    'frontend-dapp/src/utils/swapRouteSlippage.ts', 'scripts/qa/browser-live-trading-quotes-1473.cjs',
    'scripts/qa/browser-case-lifecycle.cjs',
    'scripts/qa/browser-route-attribution-1483.cjs']) {
    assert(execFileSync('git', ['show', sha + ':' + file], { cwd: ROOT, maxBuffer: 256 * 1024 })
      .equals(fs.readFileSync(path.join(ROOT, file))), 'LocalValidatorRevision');
  }
  const shared = helpers.sharedSources();
  const assets = {
    ...JSON.parse(fs.readFileSync(path.join(ROOT, 'docs/qa/issue-1473/matrix.json'))).assets,
    ...JSON.parse(fs.readFileSync(path.join(ROOT, 'docs/qa/issue-1479/matrix.json'))).assets,
  };
  const receipt = { issue: 1489, expected_revision: sha, utc_start: new Date().toISOString(),
    tool_sha256: crypto.createHash('sha256').update(fs.readFileSync(__filename)).digest('hex'),
    read_only: true, signing: false, samples: [], artifacts: [], completed: false,
    browser_budget_ms: 750, cycles_seconds: [0, 25, 185], request_families: {} };
  let browser;
  let reads = 0, writes = 0, actors = 0, providers = 0, sockets = 0;
  const output = process.env.DEX1489_BROWSER_OUTPUT || '/tmp/dex1489-browser.json';
  const save = () => fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n', { mode: 0o600 });
  const publicRead = async (url, bound) => {
    const budget = countBoundedRead(reads, MAX_READS);
    reads = budget.attempted;
    recordRequestDiagnostic(receipt.request_families, new URL(url).pathname, budget.allowed);
    assert(budget.allowed, 'ReadBudget');
    const response = await fetch(url, { redirect: 'error', signal: AbortSignal.timeout(15000) });
    assert(response.ok, 'PublicRead');
    const reader = response.body.getReader(), chunks = [];
    let size = 0;
    while (true) {
      const next = await reader.read();
      if (next.done) break;
      size += next.value.length;
      if (size > bound) { await reader.cancel(); throw new Error('BodyBudget'); }
      chunks.push(Buffer.from(next.value));
    }
    return Buffer.concat(chunks);
  };
  const attest = async () => {
    const health = JSON.parse(await publicRead(BACKEND + '/health', 4096));
    assert(health.git_sha === sha, 'BackendRevision');
    const html = await publicRead(FRONTEND + '/', 128 * 1024);
    const roots = [];
    for (const script of helpers.rootScriptPaths(html)) {
      const bytes = await publicRead(FRONTEND + script, 8 * 1024 * 1024);
      roots.push({ asset: path.posix.basename(script), path: script,
        sha256: crypto.createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length });
    }
    const actual = { html_sha256: crypto.createHash('sha256').update(html).digest('hex'), root_scripts: roots };
    helpers.checkServedArtifact(actual, artifact);
    receipt.artifacts.push(actual);
  };
  const started = Date.now();
  const timer = setTimeout(() => { void browser?.close(); }, 240000);
  timer.unref();
  try {
    await attest();
    const catalog = JSON.parse(await publicRead(BACKEND + '/api/v1/tokens/' + assets.cLUNC.identity + '/pairs', 2 * 1024 * 1024));
    assert(Array.isArray(catalog) && catalog.length <= 256, 'CatalogBound');
    const pairs = catalog.filter(pair => pair.is_active !== false && pair.asset_0 && pair.asset_1 &&
      [pair.asset_0.contract_addr, pair.asset_1.contract_addr].includes(assets.cLUNC.identity) &&
      [pair.asset_0.contract_addr, pair.asset_1.contract_addr].includes(assets.UST1.identity) &&
      /^terra1[a-z0-9]{38,80}$/.test(pair.pair_address || ''));
    assert(pairs.length === 1, 'TradeIdentity');
    browser = await frontend('@playwright/test').chromium.launch({ channel: 'chrome', headless: true,
      args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--disable-background-timer-throttling'] });
    const cases = [];
    for (const [view, viewport] of [['desktop', { width: 1440, height: 1000 }], ['phone', { width: 390, height: 844 }]]) {
      for (const entry of ['swap', 'trade']) {
        const context = await browser.newContext({ viewport, serviceWorkers: 'block' });
        await context.exposeBinding('__prohibited1483', (_, kind) => { if (kind === 'actor') actors++; else writes++; });
        await context.routeWebSocket('**/*', socket => { sockets++; socket.close(); });
        await context.addInitScript({ content: `window.__validate1483 = ${shared.validator}; window.__explain1483 = ${shared.explanation}; window.__precise1483 = ${shared.precision}; window.__format1483 = ${shared.format};` });
        await context.addInitScript(helpers.installRuntime, { copies: helpers.COPIES });
        await context.addInitScript(observeProgress);
        await context.route('**/*', route => {
          const request = route.request(), url = new URL(request.url());
          if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) { writes++; return route.abort(); }
          if (url.searchParams.has('trader') || url.searchParams.has('sender')) { actors++; return route.abort(); }
          if (url.origin === BACKEND && url.pathname.startsWith('/api/')) {
            const budget = countBoundedRead(reads, MAX_READS);
            reads = budget.attempted;
            recordRequestDiagnostic(receipt.request_families, url.pathname, budget.allowed);
            if (!budget.allowed) return route.abort();
          }
          if (/(?:cosmos|cosmwasm|abci_query|lcd|rpc)(?:\/|$)/.test(url.pathname) ||
              (['fetch', 'xhr'].includes(request.resourceType()) && ![FRONTEND, BACKEND].includes(url.origin))) {
            providers++; return route.abort();
          }
          return route.continue();
        });
        const page = await context.newPage();
        await page.goto(FRONTEND + (entry === 'swap' ? '/?from=LUNC&to=USTR' : '/trade/' + pairs[0].pair_address), { waitUntil: 'domcontentloaded', timeout: 30000 });
        const risk = page.getByRole('checkbox', { name: /I understand these risks/ });
        if (await risk.isVisible().catch(() => false)) { await risk.check(); await page.getByRole('button', { name: 'Continue to the app' }).click(); }
        const trade = entry === 'trade';
        if (trade) await page.locator('[data-testid="trade-order-tab-market"]:visible').first().click();
        const pay = page.locator(`[data-testid="${trade ? 'limit-order-escrow-amount-input' : 'swap-you-pay-amount'}"]:visible`).first();
        await pay.waitFor({ timeout: 20000 });
        if (trade) {
          await page.locator('[data-testid="trade-market-advanced-toggle"]:visible').first().click();
          await page.locator('[data-testid="trade-market-slippage-preset-5"]:visible').first().click();
          await page.locator('[data-testid="trade-market-advanced-toggle"]:visible').first().click();
        } else {
          await page.getByRole('button', { name: 'Settings', exact: true }).click();
          await page.getByTestId('swap-slippage-preset-5').click();
          await page.getByRole('button', { name: 'Settings', exact: true }).click();
        }
        await pay.fill('0.1');
        await page.waitForFunction(() => window.__quotes1473.some(row => row.useful && row.expires > Date.now()), undefined, { timeout: 15000 });
        const latest = await page.evaluate(() => window.__quotes1473.filter(row => row.useful).at(-1).params);
        if (trade && latest.token_in !== assets.cLUNC.identity) {
          const ask = page.locator('[data-testid="trade-ticket-side-ask"]:visible').first();
          const selected = await ask.getAttribute('aria-checked');
          await page.locator(`[data-testid="trade-ticket-side-${selected === 'true' ? 'bid' : 'ask'}"]:visible`).first().click();
        }
        // Prepare the viewport before starting any input clock.
        await pay.evaluate(node => node.scrollIntoView({ block: 'center' }));
        cases.push({ view, entry, context, page, pay });
      }
    }
    const phaseStart = Date.now();
    for (const seconds of [0, 25, 185]) {
      await new Promise(resolve => setTimeout(resolve, Math.max(0, phaseStart + seconds * 1000 - Date.now())));
      await Promise.all(cases.map(async ({ view, entry, page, pay }) => {
        const trade = entry === 'trade';
        const expected = { input: trade ? assets.cLUNC.identity : assets.LUNC.identity,
          output: trade ? assets.UST1.identity : assets.USTR.identity, amount: '10000000000',
          spread: '0.05', decimals: trade ? assets.UST1.decimals : assets.USTR.decimals,
          offerDecimals: 6, entry };
        await pay.fill('0');
        await page.waitForTimeout(100);
        await page.evaluate(() => { window.__progress1489 = []; });
        await page.evaluate(new Function(`return (${helpers.measurementSource(shared, entry)})`)(), expected);
        await pay.fill('10000');
        await page.waitForFunction(() => window.__measurement1473?.done != null, undefined, { timeout: 15000, polling: 10 }).catch(() => {});
        const row = await page.evaluate(() => ({ ...window.__measurement1473.read() }));
        // Observe natural adoption; do not send a best request or alter application responses.
        await page.waitForTimeout(3500);
        const observed = await page.evaluate(({ entry, expected }) => {
          const quotes = window.__quotes1473.filter(row => row.useful && row.params.token_in === expected.input &&
            row.params.token_out === expected.output && row.params.amount_in === expected.amount);
          const latest = quotes.at(-1);
          const node = [...document.querySelectorAll(`[data-testid="${entry === 'trade' ? 'trade-market-expected-receive' : 'swap-you-receive'}"]`)]
            .find(node => node.getBoundingClientRect().height > 0);
          return { quotes, progress: window.__progress1489,
            value: node ? window.__receive1483(node, entry) : null, latest };
        }, { entry, expected });
        const latest = observed.latest;
        const adopted = Boolean(latest && latest.expires > Date.now() && observed.value != null &&
          displayRaw(observed.value, expected.decimals) === latest.output);
        const progressMatches = observed.progress.length > 0 && observed.progress.every(params => matchingProgress(params, observed.quotes));
        Object.assign(row, { viewport: view, entry, cycle_seconds: seconds,
          progress_count: observed.progress.length, progress_intent_matches: progressMatches,
          validated_latest_adopted: adopted, latest_net_raw: latest?.output,
          original_generation: latest?.generation, original_observed_ms: latest?.observed,
          original_expires_ms: latest?.expires });
        row.failed = sampleFailed(row);
        // Preserve the entire 3.5s adoption observation before making this case idle.
        await recordThenClearInput(pay, () => { receipt.samples.push(row); save(); });
      }));
    }
    await attest();
    receipt.completed = receipt.samples.length === 12;
  } catch (error) {
    receipt.failure_class = /^[A-Za-z]{1,48}$/.test(error.message) ? error.message : 'BrowserFailure';
  } finally {
    clearTimeout(timer);
    await browser?.close();
    receipt.read_count = reads;
    receipt.blocked_writes = writes; receipt.actor_requests = actors;
    receipt.provider_requests = providers; receipt.websocket_requests = sockets;
    receipt.elapsed_ms = Date.now() - started;
    receipt.utc_end = new Date().toISOString();
    receipt.overall = receipt.completed && !receipt.failure_class && receipt.samples.every(row => !row.failed) &&
      writes + actors + providers + sockets === 0 && reads <= MAX_READS ? 'PASS' : 'FAIL';
    save();
    console.log(JSON.stringify({ overall: receipt.overall, samples: receipt.samples.length,
      failures: receipt.samples.filter(row => row.failed).length, failure_class: receipt.failure_class }));
  }
  return receipt.overall === 'PASS' ? 0 : 1;
}
module.exports = { intent, matchingProgress, displayRaw, sampleFailed, observeProgress, run };
if (require.main === module) run().then(code => { process.exitCode = code; }).catch(() => {
  console.error(JSON.stringify({ overall: 'FAIL', failure_class: 'PrerequisiteValidation' })); process.exitCode = 1;
});
