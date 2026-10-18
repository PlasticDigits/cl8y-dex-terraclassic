#!/usr/bin/env node
// Additional #1512 acceptance only: no wallet, signing, activity export or runtime configuration.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createRequire } = require('node:module');
const root = path.resolve(__dirname, '../..');
const { chromium, expect } = createRequire(path.join(root, 'frontend-dapp/package.json'))('@playwright/test');
const { PAIR, UST1, USTR, acknowledge, candles } = require('./browser-price-campaign-1512.cjs');
const PAIR_B = 'terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f';
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch';
// Two full document loads are necessary to verify real reload persistence.
// Playwright routing disables the HTTP asset cache; bound both script loads too.
const REQUEST_CAP = 240;
const PREFIX = { trade: 'cl8y-dex-trade-pair-invert:', charts: 'cl8y-dex-charts-pair-invert:' };
function option(name, fallback) {
  const at = process.argv.indexOf(name);
  return at < 0 ? fallback : process.argv[at + 1];
}
function market(address) {
  if (![PAIR, PAIR_B].includes(address)) throw new Error('UnknownFixturePair');
  const b = address === PAIR_B;
  return {
    pair_address: address,
    asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
    asset_1: { symbol: b ? 'cUSTC' : 'USTR', contract_addr: b ? CUSTC : USTR, denom: null, decimals: b ? 6 : 18 },
    lp_token: null, fee_bps: 30, is_active: true, code_id_frozen: false,
    volume_quote_24h: '1000000000000000000000', volume_usd_24h: '10', liquidity_usd: '1000',
  };
}
function resource(url) {
  if (url.pathname === '/api/v1/pairs') return { kind: 'catalog' };
  for (const address of [PAIR, PAIR_B]) {
    if (url.pathname === `/api/v1/pairs/${address}`) return { kind: 'pair', address };
    if (url.pathname === `/api/v1/pairs/${address}/trades`) return { kind: 'trades', address };
    if (url.pathname === `/api/v1/pairs/${address}/candles`) return { kind: 'candles', address };
  }
  return null;
}
function row(address, updated = false) {
  const p = market(address), b = address === PAIR_B;
  return {
    id: updated ? 2 : 1, pair_address: address, block_height: 1,
    block_timestamp: new Date(Date.now() - 1000).toISOString(), tx_hash: '', sender: '',
    offer_asset: 'UST1', ask_asset: p.asset_1.symbol,
    offer_amount: '1000000', return_amount: b ? '5000000' : '2000000000000000000',
    offer_decimals: 6, ask_decimals: p.asset_1.decimals, price: b ? '5' : '2',
    price_usd: b ? (updated ? '75' : '50') : '6',
  };
}
async function main() {
  const entry = option('--case', 'trade'), viewport = option('--viewport', 'desktop');
  if (!['trade', 'charts'].includes(entry) || !['desktop', 'phone'].includes(viewport)) throw new Error('InvalidCase');
  const origin = new URL(option('--origin', 'https://dex.cl8y.com')).origin;
  const backend = new URL(option('--backend', 'https://indexer.dex.cl8y.com')).origin;
  const output = option('--output', `/tmp/deployed-price-races-${entry}-${viewport}-1512.json`);
  const receipt = {
    schema: 1, issues: [1512], case: `${entry}-race-reload`, viewport,
    started_at_utc: new Date().toISOString(), verdict: 'FAIL', checks: {},
    scope: 'Served frontend; owned two-pair HTTP price fixtures, delayed completions and virtual refetch time. No live pricing, signing or backend-rollout claim.',
    browser_budget_ms: 52000, request_cap: REQUEST_CAP, requests: 0, trade_reads: 0,
    blocked_writes: 0, blocked_actor_requests: 0, request_budget_exceeded: false,
    delayed_responses: [], served_scripts: [],
    tool_digest: crypto.createHash('sha256').update(fs.readFileSync(__filename)).digest('hex'),
  };
  let browser, page, stage = 'launch', holdAddress = null, held = null, updatedB = false;
  const started = Date.now(), scripts = [], seen = new Set(), failedRequests = new Set();
  const deadline = setTimeout(() => { void browser?.close(); }, 52000);
  deadline.unref();
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true,
      args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'] });
    const context = await browser.newContext({ serviceWorkers: 'block',
      viewport: viewport === 'phone' ? { width: 390, height: 844 } : { width: 1440, height: 1000 } });
    await context.routeWebSocket('**/*', socket => socket.close());
    await context.route('**/*', async route => {
      const request = route.request(), url = new URL(request.url());
      if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) { receipt.blocked_writes++; return route.abort(); }
      if (url.searchParams.has('trader') || url.searchParams.has('sender')) { receipt.blocked_actor_requests++; return route.abort(); }
      if (++receipt.requests > REQUEST_CAP) { receipt.request_budget_exceeded = true; return route.abort(); }
      if (![origin, backend].includes(url.origin)) return route.continue();
      const hit = resource(url);
      if (!hit) return route.continue();
      let body;
      if (hit.kind === 'catalog') body = { items: [market(PAIR), market(PAIR_B)], total: 2, limit: 100, offset: 0 };
      if (hit.kind === 'pair') body = market(hit.address);
      if (hit.kind === 'candles') body = candles();
      if (hit.kind === 'trades') {
        receipt.trade_reads++;
        body = [row(hit.address, updatedB)];
        if (hit.address === holdAddress) {
          holdAddress = null;
          const evidence = { phase: hit.address === PAIR ? 'old-pair-after-switch' : 'same-pair-after-invert', fulfilled: false, cancelled: false,
            browser_status: null, body_complete: false, request_failed: false };
          receipt.delayed_responses.push(evidence);
          return new Promise(resolve => {
            held = async () => {
              try {
                await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
                // Chromium may silently discard Fetch.fulfillRequest after
                // cancellation. Bind proof to this actual Request's response
                // and completed body, rather than the interception API ACK.
                const response = await request.response();
                evidence.browser_status = response?.status() ?? null;
                evidence.body_complete = !!response && await response.finished() === null;
                evidence.request_failed = failedRequests.has(request) || request.failure() !== null;
                evidence.cancelled = !response || evidence.request_failed;
                evidence.fulfilled = evidence.browser_status === 200 && evidence.body_complete && !evidence.cancelled;
              } catch { evidence.cancelled = true; }
              finally { resolve(); }
            };
          });
        }
      }
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
    });
    page = await context.newPage();
    page.on('requestfailed', request => failedRequests.add(request));
    page.setDefaultTimeout(5000);
    await page.clock.install(); // Own timers BEFORE React Query installs refetch intervals.
    page.on('response', response => {
      const url = new URL(response.url());
      if (url.origin !== origin || !/^\/assets\/[^/]+\.js$/.test(url.pathname) || seen.has(url.pathname)) return;
      seen.add(url.pathname);
      scripts.push(response.body().then(bytes => {
        if (bytes.length > 8 * 1024 * 1024) throw new Error('ScriptBodyBudget');
        receipt.served_scripts.push({ path: url.pathname, sha256: crypto.createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length });
      }));
    });
    const headline = () => page.locator('[data-testid="trade-chart-headline-price"]:visible').first().locator('span').last();
    const pill = () => page.locator('[data-testid="trade-pair-invert-pill"]:visible').first();
    const value = async expected => expect.poll(async () => Number((await headline().textContent()).replace(/,/g, ''))).toBe(expected);
    const pauseRefetch = async address => {
      holdAddress = address; held = null;
      await page.clock.fastForward(16000);
      await expect.poll(() => !!held).toBe(true);
    };
    const release = async () => {
      const fn = held; held = null;
      if (!fn) throw new Error('MissingOwnedDelayedResponse');
      await fn();
      // Drain the delivered response/application effects without advancing a new refetch cycle.
      await page.clock.runFor(150);
    };
    const select = async address => {
      const label = address === PAIR ? /USTR/ : /cUSTC/;
      await page.getByRole(entry === 'trade' ? 'combobox' : 'button', { name: entry === 'trade' ? 'Trading pair' : 'Select pair', exact: true }).click();
      await page.getByRole('option').filter({ hasText: label }).first().click();
      await expect.poll(() => new URL(page.url()).pathname).toBe(`/${entry}/${address}`);
    };
    stage = 'initial-owned-price';
    await page.goto(`${origin}/${entry}/${PAIR}`, { waitUntil: 'domcontentloaded', timeout: 18000 });
    await acknowledge(page);
    await value(entry === 'trade' ? 3 : 6);
    stage = 'hold-old-pair-response';
    await pauseRefetch(PAIR);
    stage = 'invert-while-old-pair-pending';
    await pill().click();
    await value(entry === 'trade' ? 6 : 3);
    receipt.checks.orientation_change_while_response_pending = true;
    stage = 'select-new-pair-before-old-response';
    await select(PAIR_B);
    await value(entry === 'trade' ? 10 : 50);
    receipt.checks.new_pair_has_own_default_orientation = true;
    stage = 'old-response-after-new-pair';
    await release();
    await value(entry === 'trade' ? 10 : 50);
    await expect(pill()).toContainText('cUSTC');
    receipt.checks.late_old_pair_cannot_replace_new_price_or_label = true;
    stage = 'hold-new-pair-updated-response';
    updatedB = true;
    await pauseRefetch(PAIR_B);
    stage = 'invert-before-new-response';
    await pill().click();
    await value(entry === 'trade' ? 50 : 10);
    await release();
    await value(entry === 'trade' ? 75 : 15);
    receipt.checks.late_same_pair_response_uses_current_orientation_once = true;
    stage = 'actual-page-reload';
    // Charts encodes orientation in ?price=. Remove it before the real reload
    // so success must come from session persistence, not a carried URL override.
    await page.evaluate(() => {
      const url = new URL(location.href); url.searchParams.delete('price');
      history.replaceState(history.state, '', url.pathname + url.search);
    });
    await page.reload({ waitUntil: 'domcontentloaded', timeout: 15000 });
    await value(entry === 'trade' ? 75 : 15);
    await expect(pill()).toContainText('cUSTC');
    receipt.checks.reload_preserves_pair_orientation_and_indexed_price = true;
    stage = 'clean-url-session-persistence';
    const stored = await page.evaluate(({ own, other, address }) => ({
      own: sessionStorage.getItem(own + address), other: sessionStorage.getItem(other + address),
    }), { own: PREFIX[entry], other: PREFIX[entry === 'trade' ? 'charts' : 'trade'], address: PAIR_B });
    if (stored.own !== (entry === 'trade' ? '0' : '1') || stored.other !== null) throw new Error('OrientationNamespaceMismatch');
    receipt.checks.clean_url_uses_own_persisted_orientation = true;
    receipt.checks.other_entry_orientation_namespace_untouched = true;
    stage = 'return-to-first-pair';
    await select(PAIR);
    await value(entry === 'trade' ? 6 : 3);
    await expect(pill()).toContainText('USTR');
    receipt.checks.return_to_first_pair_restores_its_own_orientation = true;
    await Promise.all(scripts);
    if (receipt.request_budget_exceeded || receipt.blocked_writes || receipt.blocked_actor_requests || !receipt.served_scripts.length ||
        receipt.delayed_responses.length !== 2 || receipt.delayed_responses.some(r => !r.fulfilled))
      throw new Error('BoundedAnonymousCompletionInvariant');
    receipt.verdict = 'PASS';
  } catch { receipt.failed_stage = stage; }
  finally {
    clearTimeout(deadline);
    await browser?.close();
    receipt.elapsed_ms = Date.now() - started;
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n', { mode: 0o600 });
    process.stdout.write(JSON.stringify({ verdict: receipt.verdict, case: receipt.case, viewport, elapsed_ms: receipt.elapsed_ms, output }) + '\n');
    if (receipt.verdict !== 'PASS') process.exitCode = 1;
  }
}
if (require.main === module) main().catch(() => { process.exitCode = 1; });
module.exports = { PAIR, PAIR_B, PREFIX, REQUEST_CAP, market, resource, row };
