#!/usr/bin/env node
// One read-only actual-market pass; exports only price scalars/booleans and served asset hashes.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createRequire } = require('node:module');
const { PAIR, UST1, USTR, acknowledge } = require('./browser-price-campaign-1512.cjs');
const root = path.resolve(__dirname, '../..');
const { chromium, expect } = createRequire(path.join(root, 'frontend-dapp/package.json'))('@playwright/test');
const ORIGIN = 'https://dex.cl8y.com';
const BACKEND = 'https://indexer.dex.cl8y.com';
function positive(value) {
  if (typeof value !== 'string' || value.length > 128 ||
      !/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value.trim())) return null;
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? number : null;
}
function projectPair(body) {
  return body?.pair_address === PAIR && body.asset_0?.contract_addr === UST1 &&
    body.asset_1?.contract_addr === USTR && body.asset_0.decimals === 6 && body.asset_1.decimals === 18;
}
function projectTrade(body, now = Date.now()) {
  const row = Array.isArray(body) ? body[0] : null;
  if (!row) return { available: false, usd: null, human: null, binds_pair: true, timestamp_valid: true };
  let human = positive(row.price);
  if (row.price == null || row.price === '') {
    if ([row.offer_amount, row.return_amount].every(raw => typeof raw === 'string' && /^\d{1,39}$/.test(raw) && BigInt(raw) <= (1n << 128n) - 1n)) {
      const offer = BigInt(row.offer_amount), returned = BigInt(row.return_amount);
      if (offer > 0n && returned > 0n) {
        if (row.offer_asset === 'UST1' && row.ask_asset === 'USTR')
          human = Number(returned * 1000000n) / Number(offer * 1000000000000000000n);
        else if (row.offer_asset === 'USTR' && row.ask_asset === 'UST1')
          human = Number(offer * 1000000n) / Number(returned * 1000000000000000000n);
      }
    }
  }
  const timestamp = typeof row.block_timestamp === 'string' ? Date.parse(row.block_timestamp) : NaN;
  const timestampValid = Number.isFinite(timestamp) && timestamp <= now;
  const bindsPair = row.pair_address === PAIR;
  const usd = bindsPair && timestampValid ? positive(row.price_usd) : null;
  return { available: usd !== null, usd, human: Number.isFinite(human) && human > 0 ? human : null,
    binds_pair: bindsPair, timestamp_valid: timestampValid };
}
function projectCandles(body) {
  if (!Array.isArray(body)) return null;
  const all = (row, prefix) => ['open', 'high', 'low', 'close'].every(key => positive(row[prefix + key]) !== null);
  return body.slice(0, 200).map(row => ({
    timestamp: Number.isFinite(Date.parse(row.open_time)),
    subject: row.usd_leg === 'asset_1', usd: all(row, ''),
    subject_usd: all(row, 'subject_'),
    human_valid: ['open', 'high', 'low', 'close'].every(key => positive(row[key + '_human']) !== null),
  }));
}
function plottable(rows, inverted) {
  return rows.filter(row => row.timestamp && (row.subject
    ? row.subject_usd && (inverted || row.human_valid)
    : row.usd && (!inverted || row.human_valid))).length;
}
function matchesScalar(text, expected) {
  if (expected === null) return text.trim() === '—';
  const actual = Number(text.replace(/,/g, '').trim());
  return Number.isFinite(actual) && actual > 0 && Math.abs(actual - expected) <= Math.abs(expected) * 0.000006;
}
async function main() {
  const i = process.argv.indexOf('--output');
  const output = i < 0 ? '/tmp/deployed-unmocked-price-1512.json' : process.argv[i + 1];
  const receipt = { schema: 1, issue: 1512, started_at_utc: new Date().toISOString(),
    scope: 'Actual anonymous unmocked deployed Trade/Charts and their own API response price scalars; no wallet, signing, actor/tape export, screenshots or virtual clock.',
    verdict: 'FAIL', checks: [], served_scripts: [], requests: 0, blocked_writes: 0, blocked_actor_requests: 0,
    browser_budget_ms: 52000, tool_digest: crypto.createHash('sha256').update(fs.readFileSync(__filename)).digest('hex') };
  let browser, stage = 'launch';
  const started = Date.now(), scriptTasks = [], seen = new Set();
  const deadline = setTimeout(() => { void browser?.close(); }, 52000);
  deadline.unref();
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true,
      args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'] });
    const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' });
    await context.routeWebSocket('**/*', socket => socket.close());
    await context.route('**/*', route => {
      const req = route.request(), url = new URL(req.url());
      if (!['GET', 'HEAD', 'OPTIONS'].includes(req.method())) { receipt.blocked_writes++; return route.abort(); }
      if (url.searchParams.has('trader') || url.searchParams.has('sender')) { receipt.blocked_actor_requests++; return route.abort(); }
      if (++receipt.requests > 180) return route.abort();
      return route.continue(); // No response is mocked or modified.
    });
    for (const entry of ['trade', 'charts']) {
      const page = await context.newPage();
      page.setDefaultTimeout(6000);
      const projection = { pair: null, trade: null, candles: null };
      const pending = [];
      page.on('response', response => {
        const url = new URL(response.url());
        if (url.origin === ORIGIN && /^\/assets\/[^/]+\.js$/.test(url.pathname) && !seen.has(url.pathname)) {
          seen.add(url.pathname);
          scriptTasks.push(response.body().then(bytes => {
            if (bytes.length > 8 * 1024 * 1024) throw new Error('ScriptBudget');
            receipt.served_scripts.push({ path: url.pathname, sha256: crypto.createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length });
          }));
        }
        if (![ORIGIN, BACKEND].includes(url.origin) || !response.ok()) return;
        const resource = url.pathname === `/api/v1/pairs/${PAIR}` ? 'pair'
          : url.pathname === `/api/v1/pairs/${PAIR}/trades` ? 'trade'
          : url.pathname === `/api/v1/pairs/${PAIR}/candles` ? 'candles' : null;
        if (!resource) return;
        pending.push(response.body().then(bytes => {
          if (bytes.length > 2 * 1024 * 1024) throw new Error('ApiBodyBudget');
          const body = JSON.parse(bytes.toString());
          // Drop all original fields immediately after projecting only required scalars/flags.
          projection[resource] = resource === 'pair' ? projectPair(body)
            : resource === 'trade' ? projectTrade(body) : projectCandles(body);
        }));
      });
      stage = entry + '-navigate';
      await page.goto(`${ORIGIN}/${entry}/${PAIR}`, { waitUntil: 'domcontentloaded', timeout: 18000 });
      await acknowledge(page);
      const headline = page.locator('[data-testid="trade-chart-headline-price"]:visible').first();
      await expect(headline).toBeVisible({ timeout: 18000 });
      await expect.poll(() => projection.pair === true && projection.trade !== null && projection.candles !== null).toBe(true);
      await Promise.all(pending);
      const pill = page.locator('[data-testid="trade-pair-invert-pill"]:visible').first();
      const inverted = (await pill.innerText()).trim().startsWith('USTR/UST1');
      const expected = projection.trade.usd === null ? null
        : inverted ? projection.trade.human === null ? null : projection.trade.usd / projection.trade.human
        : projection.trade.usd;
      const value = headline.locator('span').last();
      stage = entry + '-api-scalar-comparison';
      await expect.poll(async () => matchesScalar(await value.innerText(), expected)).toBe(true);
      const region = headline.locator('xpath=ancestor::section[@role="region"][1]');
      const summary = region.locator(':scope > p[aria-live="polite"]');
      const plotCount = async () => Number((await summary.innerText()).match(/(\d+) candles on chart\./)?.[1] || 0);
      const available = plottable(projection.candles, inverted);
      if (available > 0) {
        await expect.poll(plotCount).toBe(available);
        await expect.poll(() => region.locator('canvas').count()).toBeGreaterThan(0);
      }
      const plotted = await plotCount();
      receipt.checks.push({ entry, inverted, api_factory_usd: projection.trade.usd,
        api_human_ratio: projection.trade.human, expected_display_usd: expected,
        expected_dash: expected === null, displayed_matches_api_scalar: true,
        pair_binding_valid: projection.trade.binds_pair, observed_timestamp_valid: projection.trade.timestamp_valid,
        historical_plottable_candles: available, historical_plotted_candles: plotted,
        historical_check_available: available > 0 });
      await page.close();
    }
    await Promise.all(scriptTasks);
    if (receipt.blocked_writes || receipt.blocked_actor_requests || !receipt.served_scripts.length)
      throw new Error('AnonymousReadOnlyInvariant');
    receipt.verdict = receipt.checks.every(row => row.historical_check_available) ? 'PASS' : 'PASS_WITH_SKIPS';
  } catch {
    receipt.failed_stage = stage;
  } finally {
    clearTimeout(deadline);
    await browser?.close();
    receipt.elapsed_ms = Date.now() - started;
    fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n', { mode: 0o600 });
    process.stdout.write(JSON.stringify({ verdict: receipt.verdict, elapsed_ms: receipt.elapsed_ms, output }) + '\n');
    if (receipt.verdict === 'FAIL') process.exitCode = 1;
  }
}
if (require.main === module) main().catch(() => { process.exitCode = 1; });
module.exports = { positive, projectPair, projectTrade, projectCandles, plottable, matchesScalar };
