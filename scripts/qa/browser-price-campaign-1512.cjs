#!/usr/bin/env node
// Bounded disconnected deployed-frontend acceptance. No signing or raw activity export.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createRequire } = require('node:module');
const root = path.resolve(__dirname, '../..');
const { chromium, expect } = createRequire(path.join(root, 'frontend-dapp/package.json'))('@playwright/test');
const PAIR = 'terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy';
const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72';
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv';
const pair = {
  pair_address: PAIR,
  asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
  asset_1: { symbol: 'USTR', contract_addr: USTR, denom: null, decimals: 18 },
  lp_token: null, fee_bps: 30, is_active: true, code_id_frozen: false,
  volume_quote_24h: '1000000000000000000000', volume_usd_24h: '10', liquidity_usd: '1000',
};
function option(name, fallback) {
  const i = process.argv.indexOf(name);
  return i < 0 ? fallback : process.argv[i + 1];
}
function fixtureKind(url) {
  if (url.pathname === '/api/v1/pairs') return 'catalog';
  if (url.pathname === `/api/v1/pairs/${PAIR}`) return 'pair';
  if (url.pathname === `/api/v1/pairs/${PAIR}/trades`) return 'trades';
  if (url.pathname === `/api/v1/pairs/${PAIR}/candles`) return 'candles';
  return null;
}
function trade(phase) {
  return {
    id: 1, pair_address: phase === 'foreign' ? 'terra1foreign-owned-fixture' : PAIR,
    block_height: 1, block_timestamp: new Date(Date.now() - 1000).toISOString(),
    tx_hash: '', sender: '', offer_asset: 'UST1', ask_asset: 'USTR',
    offer_amount: '1000000', return_amount: '160756280899197382191',
    offer_decimals: 6, ask_decimals: 18, price: '160.756280899197382191',
    price_usd: phase === 'missing' ? null : '0.849756570385773539',
  };
}
function candles() {
  const hour = Math.floor(Date.now() / 3600000) * 3600000;
  return [2, 1].map(h => ({
    open_time: new Date(hour - h * 3600000).toISOString(), usd_leg: 'asset_0',
    open: '0.8', high: '0.9', low: '0.7', close: '0.85',
    open_human: '160', high_human: '170', low_human: '150', close_human: '160.756280899197382191',
    volume_base: '1000000', volume_quote: '160756280899197382191', trade_count: 1,
  }));
}
async function acknowledge(page) {
  const checkbox = page.getByRole('checkbox', { name: /I understand these risks/ });
  const shown = await checkbox.waitFor({ state: 'visible', timeout: 5000 }).then(() => true, () => false);
  if (shown) {
    await checkbox.check();
    await page.getByRole('button', { name: 'Continue to the app' }).click();
  }
}
async function main() {
  const entry = option('--case', 'trade');
  const view = option('--viewport', 'desktop');
  if (!['trade', 'charts', 'campaign'].includes(entry) || !['desktop', 'phone'].includes(view))
    throw new Error('InvalidCase');
  const origin = new URL(option('--origin', 'https://dex.cl8y.com')).origin;
  const backend = new URL(option('--backend', 'https://indexer.dex.cl8y.com')).origin;
  const output = option('--output', `/tmp/deployed-${entry}-${view}-1512.json`);
  const receipt = {
    schema: 1, issues: entry === 'campaign' ? [1330] : [1512], case: entry, viewport: view,
    started_at_utc: new Date().toISOString(), verdict: 'FAIL', checks: {}, served_scripts: [],
    scope: entry === 'campaign'
      ? 'Deployed anonymous Swap; solver failure injected, actual backend allowed after one retry; no signing.'
      : 'Deployed frontend rendering with owned anonymous canonical-pair HTTP fixtures; not live market-price or execution acceptance.',
    browser_budget_ms: 52000, refetches: [], solver_requests: 0, forced_solver_failures: 0, solver_statuses: [], requests: 0, blocked_writes: 0, blocked_actor_requests: 0,
    tool_digest: crypto.createHash('sha256').update(fs.readFileSync(__filename)).digest('hex'),
    deployment_attestation: 'Served script paths/hashes identify observed frontend; source revision must be independently attested.',
  };
  let browser, page, stage = 'launch', phase = 'valid', recover = false, tradeReads = 0;
  const scriptTasks = [], scriptPaths = new Set();
  const started = Date.now();
  const deadline = setTimeout(() => { void browser?.close(); }, 52000);
  deadline.unref();
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true,
      args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'] });
    const context = await browser.newContext({
      viewport: view === 'phone' ? { width: 390, height: 844 } : { width: 1440, height: 1000 },
      serviceWorkers: 'block',
    });
    await context.routeWebSocket('**/*', socket => socket.close());
    await context.route('**/*', async route => {
      const request = route.request();
      const url = new URL(request.url());
      if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) {
        receipt.blocked_writes++;
        return route.abort();
      }
      if (url.searchParams.has('trader') || url.searchParams.has('sender')) {
        receipt.blocked_actor_requests++;
        return route.abort();
      }
      if (++receipt.requests > 180) return route.abort();
      if (url.origin !== backend && url.origin !== origin) return route.continue();
      if (entry === 'campaign') {
        if (url.pathname === '/api/v1/route/solve') {
          receipt.solver_requests++;
          if (['utm_source', 'utm_campaign', 'ref'].some(key => url.searchParams.has(key)))
            throw new Error('CampaignEscapedUrl');
          if (!recover) {
            receipt.forced_solver_failures++;
            return route.fulfill({ status: 503, contentType: 'application/json',
              body: JSON.stringify({ error: 'Pricing service unavailable' }) });
          }
        }
        return route.continue();
      }
      const kind = fixtureKind(url);
      if (!kind) return route.continue();
      let body;
      if (kind === 'catalog') body = { items: [pair], total: 1, limit: 100, offset: 0 };
      if (kind === 'pair') body = pair;
      if (kind === 'trades') { tradeReads++; body = [trade(phase)]; }
      if (kind === 'candles') body = candles();
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
    });
    page = await context.newPage();
    page.setDefaultTimeout(6000);
    // Own the application timers from startup; clocks installed after mount
    // cannot reliably advance already registered native React Query intervals.
    if (entry !== 'campaign') await page.clock.install();
    page.on('response', response => {
      const url = new URL(response.url());
      if ([origin, backend].includes(url.origin) && url.pathname === '/api/v1/route/solve') {
        if (receipt.solver_statuses.length < 16) receipt.solver_statuses.push(response.status());
      }
      if (url.origin !== origin || !/^\/assets\/[^/]+\.js$/.test(url.pathname) || scriptPaths.has(url.pathname)) return;
      scriptPaths.add(url.pathname);
      scriptTasks.push(response.body().then(bytes => {
        if (bytes.length > 8 * 1024 * 1024) throw new Error('ScriptBodyBudget');
        receipt.served_scripts.push({ path: url.pathname,
          sha256: crypto.createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length });
      }));
    });
    stage = 'navigate';
    await page.goto(origin + (entry === 'campaign'
      ? '/?INPUTCURRENCY=UST1&OUTPUTCURRENCY=USTR&AMOUNT=0.1&utm_source=launch&utm_source=agent&utm_campaign=middle%20hop&ref=agent'
      : `/${entry}/${PAIR}`), { waitUntil: 'domcontentloaded', timeout: 18000 });
    await acknowledge(page);
    if (entry === 'campaign') {
      stage = 'forced-solver-failure';
      const failure = page.locator('[data-testid="swap-quote-retry-error"], [data-testid="swap-market-data-outage-banner"]').filter({ has: page.getByRole('button', { name: 'Retry', exact: true }) }).first();
      await expect(failure).toBeVisible({ timeout: 18000 });
      if (!receipt.forced_solver_failures) throw new Error('SolverFailureNotObserved');
      const checkCampaign = () => {
        const search = new URL(page.url()).searchParams;
        if (JSON.stringify(search.getAll('utm_source')) !== JSON.stringify(['launch', 'agent']) ||
            search.get('utm_campaign') !== 'middle hop' || search.get('ref') !== 'agent')
          throw new Error('CampaignLost');
      };
      checkCampaign();
      receipt.checks.campaign_survives_failure = true;
      stage = 'one-action-retry';
      const beforeRetry = receipt.solver_requests;
      recover = true;
      await failure.getByRole('button', { name: 'Retry', exact: true }).click();
      await expect.poll(() => receipt.solver_requests, { timeout: 18000 }).toBeGreaterThan(beforeRetry);
      await expect(failure).toHaveCount(0, { timeout: 18000 });
      await expect.poll(async () => {
        const n = Number(await page.getByTestId('swap-you-receive').inputValue());
        return Number.isFinite(n) && n > 0;
      }, { timeout: 18000 }).toBe(true);
      checkCampaign();
      receipt.checks.one_click_retry_recovers = true;
      receipt.checks.campaign_survives_retry = true;
    } else {
      const headline = page.locator('[data-testid="trade-chart-headline-price"]:visible').first();
      const region = headline.locator('xpath=ancestor::section[@role="region"][1]');
      const first = entry === 'trade' ? '0.00528599' : '0.849757';
      const flipped = entry === 'trade' ? '0.849757' : '0.00528599';
      stage = 'initial-indexed-usd';
      await expect(headline).toContainText(first, { timeout: 18000 });
      await expect(region.locator(':scope > p[aria-live="polite"]')).toContainText('2 candles on chart.');
      receipt.checks.initial_indexed_usd_matches = true;
      receipt.checks.initial_historical_candles = true;
      const refetch = async (next, expected) => {
        phase = next;
        const before = tradeReads;
        const sample = { phase: next, before, after: before };
        receipt.refetches.push(sample);
        stage = next + '-wait-for-refetch';
        try {
          await page.clock.fastForward(16000);
          await expect.poll(() => tradeReads, { timeout: 5000 }).toBeGreaterThan(before);
          stage = next + '-headline-assertion';
          await expect(headline).toContainText(expected);
        } finally { sample.after = tradeReads; }
      };
      stage = 'missing-current-usd';
      await refetch('missing', '—');
      await expect(region.locator(':scope > p[aria-live="polite"]')).toContainText('Last price unavailable.');
      await expect(region.locator(':scope > p[aria-live="polite"]')).toContainText('2 candles on chart.');
      receipt.checks.missing_current_usd_is_dash = true;
      receipt.checks.historical_candles_retained = true;
      stage = 'invert-while-unpriced';
      await page.locator('[data-testid="trade-pair-invert-pill"]:visible').first().click();
      await expect(headline).toContainText('—');
      receipt.checks.inversion_keeps_dash = true;
      stage = 'indexed-usd-recovery';
      await refetch('valid', flipped);
      receipt.checks.indexed_usd_recovers_inverted = true;
      stage = 'foreign-pair-rejection';
      await refetch('foreign', '—');
      receipt.checks.foreign_pair_usd_rejected = true;
    }
    await Promise.all(scriptTasks);
    if (receipt.blocked_writes || receipt.blocked_actor_requests || !receipt.served_scripts.length)
      throw new Error('AnonymousReadOnlyInvariant');
    receipt.verdict = 'PASS';
  } catch {
    receipt.failed_stage = stage;
    if (entry === 'campaign' && page && !page.isClosed()) {
      receipt.ui_diagnostic = await page.evaluate(() => {
        const value = selector => document.querySelector(selector)?.value;
        return {
          pay_prefill_matched: value('[aria-label="Select token you pay"]') === 'UST1',
          receive_prefill_matched: value('[aria-label="Select token you receive"]') === 'USTR',
          amount_prefill_matched: value('[data-testid="swap-you-pay-amount"]') === '0.1',
          retry_error_present: !!document.querySelector('[data-testid="swap-quote-retry-error"]'),
          outage_banner_present: !!document.querySelector('[data-testid="swap-market-data-outage-banner"]'),
        };
      }).catch(() => ({ unavailable: true }));
    }
  } finally {
    clearTimeout(deadline);
    await browser?.close();
    receipt.elapsed_ms = Date.now() - started;
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n', { mode: 0o600 });
    process.stdout.write(JSON.stringify({ verdict: receipt.verdict, case: entry, viewport: view,
      elapsed_ms: receipt.elapsed_ms, output }) + '\n');
    if (receipt.verdict !== 'PASS') process.exitCode = 1;
  }
}
if (require.main === module) main().catch(() => { process.exitCode = 1; });
module.exports = { PAIR, UST1, USTR, acknowledge, fixtureKind, trade, candles };
