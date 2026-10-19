// Read-only live browser check. No wallet injection, signing or transaction calls.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const frontend = createRequire(path.resolve(__dirname, '../../frontend-dapp/package.json'));
const { chromium } = frontend('@playwright/test');
const AxeBuilder = frontend('@axe-core/playwright').default;
async function seriousViolations(page) {
  const scan = await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21a','wcag21aa'])
    .exclude('[data-testid="price-chart-lightweight-canvas"] canvas').analyze();
  return scan.violations.filter(v => ['critical','serious'].includes(v.impact)).length;
}
const output = process.argv[2];
if (!output) throw new Error('OutputPathRequired');
const receipt = { issues: [1560, 1561], utc: new Date().toISOString(), scope: 'Live disconnected Swap with actual cached native quote; Trade market preview with actual cached wrapped quote. No funded settlement or physical device proof.', observations: [] };
const check = (condition, code) => { if (!condition) throw new Error(code); };
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true,
    args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'] });
  try {
    for (const viewport of [{width: 375, height: 667}, {width: 1440, height: 1000}]) {
      const context = await browser.newContext({ viewport });
      const page = await context.newPage();
      let errors = 0;
      page.on('pageerror', () => errors++);
      const responsePromise = page.waitForResponse(r => r.url().startsWith('https://indexer.dex.cl8y.com/api/v1/route/solve?') && new URL(r.url()).searchParams.get('amount_in') === '100000000' && r.status() === 200, {timeout: 15000});
      await page.goto('https://dex.cl8y.com/?from=uusd&to=terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3&amount=100', {waitUntil: 'domcontentloaded', timeout: 15000});
      const q = await (await responsePromise).json();
      check(q.token_in === 'uusd' && q.token_out === 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3' && q.execution_metadata?.input_amount === '100000000', 'OriginalEvidenceUnavailable');
      const d = q.order_economics;
      check(d?.status === 'available' && q.economic_attribution?.status === 'available', 'OriginalEvidenceUnavailable');
      const movement = BigInt(d.order_movement), baseline = BigInt(d.baseline_amount_out);
      const fair = BigInt(q.spot_amount_out), net = BigInt(q.estimated_amount_out);
      const delta = fair > net ? fair - net : net - fair, denominator = fair > net ? fair : net;
      const expectedNotice = movement * 100n > baseline * 5n || delta * 100n > denominator * 10n;
      await page.locator('[data-testid="swap-you-receive"]').waitFor();
      const riskAcknowledgement = page.getByRole('checkbox', {name: /I understand these risks/});
      if (await riskAcknowledgement.isVisible()) {
        await riskAcknowledgement.check();
        await page.getByRole('button', {name: 'Continue to the app', exact: true}).click();
      }
      await page.waitForFunction(() => {
        const el = document.querySelector('[data-testid="swap-you-receive"]');
        return el && el.value !== '' && el.value !== '0' && !document.querySelector('[data-testid="swap-price-data-loading"]');
      }, null, {timeout: 10000});
      const initialOutput = await page.getByTestId('swap-you-receive').inputValue();
      const initialParts = initialOutput.split('.'), initialFraction = initialParts[1] || '';
      const digits = BigInt(initialParts[0] + initialFraction);
      const scales = Array.from({length:39},(_,d)=>d).filter(d => d >= initialFraction.length && digits * 10n**BigInt(d-initialFraction.length) === BigInt(q.estimated_amount_out));
      check(scales.length === 1, 'LiveQuoteScaleMismatch');
      const outputDecimals = scales[0];
      await page.evaluate(() => window.scrollTo(0, 0));
      const measured = await page.evaluate(() => {
        const cta = document.querySelector('[data-testid="swap-submit"]');
        const nav = document.querySelector('.app-mobile-nav-shell');
        const status = document.querySelector('[data-testid="swap-quote-status"]');
        const route = document.querySelector('[data-testid="swap-route-summary"]');
        const box = cta.getBoundingClientRect(), navBox = nav?.getBoundingClientRect();
        const segments = [...route.querySelectorAll('.swap-route-path > :not(.sr-only)')].map(el => {
          const first = el.hasAttribute('data-token-identity') ? el.firstElementChild : el;
          const last = el.hasAttribute('data-token-identity') ? el.lastElementChild : el;
          return {left:first.getBoundingClientRect().left, right:last.getBoundingClientRect().right};
        });
        const routeRightGap = segments.length ? route.getBoundingClientRect().right - segments.at(-1).right : 0;
        const routeMaxGap = Math.max(0,...segments.slice(1).map((s,i)=>s.left-segments[i].right));
        const v = window.visualViewport;
        const statusStyle = getComputedStyle(status);
        const form = document.querySelector('.swap-page').getBoundingClientRect();
        const headerBottom = document.querySelector('.app-top-sticky').getBoundingClientRect().bottom;
        const footerDisplayed = getComputedStyle(document.querySelector('.app-footer-shell')).display !== 'none';
        const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
        return {cta: {top: box.top, bottom: box.bottom, height: box.height}, navTop: navBox?.height ? navBox.top : innerHeight,
          viewportBottom: v ? v.offsetTop + v.height : innerHeight, scrollY,
          form: {top: form.top, bottom: form.bottom}, headerBottom, footerDisplayed,
          centerHit: hit === cta || cta.contains(hit), pageOverflow: document.documentElement.scrollWidth > innerWidth,
          route: route && {scrollWidth: route.scrollWidth, clientWidth: route.clientWidth, rightGap:routeRightGap, maxGap:routeMaxGap},
          status: {scrollWidth: status.scrollWidth, clientWidth: status.clientWidth, empty: !status.textContent.trim(), font:parseFloat(statusStyle.fontSize), lineHeight:parseFloat(statusStyle.lineHeight), minHeight:parseFloat(statusStyle.minHeight)},
          riskNoticeCount: document.querySelectorAll('[data-testid="swap-impact-aside"]').length,
          detailsCount: document.querySelectorAll('[data-testid="swap-details"]').length,
          renderedRouteTokens: route?.querySelectorAll('[data-token-identity]').length};
      });
      check(measured.riskNoticeCount === Number(expectedNotice), 'LiveRiskNoticeMismatch');
      if (!expectedNotice) check(measured.status.empty, 'LiveRedundantStatus');
      check(measured.status.lineHeight / measured.status.font <= 1.4 && measured.status.minHeight / measured.status.font <= 2.71, 'LiveStatusDensity');
      check(!measured.pageOverflow && measured.route && measured.route.scrollWidth <= measured.route.clientWidth + 1 && measured.status.scrollWidth <= measured.status.clientWidth + 1, 'LiveHorizontalOverflow');
      check(measured.cta.top >= 0 && measured.cta.bottom <= Math.min(measured.navTop, measured.viewportBottom) && measured.centerHit, 'LiveCTAObscured');
      check(Math.abs(measured.route.rightGap) <= 1 && measured.route.maxGap <= 5, 'LiveRouteSpacing');
      check(measured.footerDisplayed === (viewport.width >= 768), 'LiveFooterVisibility');
      if (viewport.width < 768) check(Math.abs((measured.form.top + measured.form.bottom - measured.headerBottom - measured.navTop) / 2) <= 1, 'LiveFormNotCentered');
      check(measured.detailsCount === 1 && measured.renderedRouteTokens >= 3, 'NativeRouteOrDisclosureMissing');
      measured.seriousOrCriticalAxe = await seriousViolations(page);
      check(measured.seriousOrCriticalAxe === 0, 'LiveAccessibilityFailure');
      check(errors === 0, 'LiveRuntimeError');
      receipt.observations.push({viewport, expectedRiskNotice: expectedNotice, ...measured, runtimeErrors: errors});
      if (process.env.TRADING_UI_SCREENSHOT_DIR) {
        fs.mkdirSync(process.env.TRADING_UI_SCREENSHOT_DIR, {recursive: true});
        await page.screenshot({path: path.join(process.env.TRADING_UI_SCREENSHOT_DIR, `live-${viewport.width}x${viewport.height}.png`)});
      }
      // Flip only the anonymous preview. No wallet connection or submission.
      const displayedOutput = await page.getByTestId('swap-you-receive').inputValue();
      const parts = displayedOutput.split('.'), fraction = parts[1] || '';
      const reverseRaw = (BigInt(parts[0] + fraction) * 10n**BigInt(outputDecimals-fraction.length)).toString();
      const reverseReply = page.waitForResponse(r => r.url().startsWith('https://indexer.dex.cl8y.com/api/v1/route/solve?') && new URL(r.url()).searchParams.get('token_in') === q.token_out && new URL(r.url()).searchParams.get('amount_in') === reverseRaw, {timeout: 15000});
      await page.getByRole('button', {name:'Swap pay and receive tokens', exact:true}).click();
      await page.waitForFunction(expected => document.querySelector('[data-testid="swap-you-pay-amount"]').value === expected, displayedOutput, {timeout:5000});
      const reverseResponse = await reverseReply;
      check(reverseResponse.status() === 200, 'LiveReverseQuoteUnavailable');
      check(errors === 0, 'LiveRuntimeError');
      receipt.observations.at(-1).directionSwitch = {previousOutput:displayedOutput, newPayMatches:true, reverseRequestRaw:reverseRaw, reverseHttp:reverseResponse.status(), signingAttempts:0};
      if (process.env.TRADING_UI_SCREENSHOT_DIR) await page.screenshot({path:path.join(process.env.TRADING_UI_SCREENSHOT_DIR, `live-direction-${viewport.width}x${viewport.height}.png`)});
      if (viewport.width === 1440) {
        const wrapped = q.execution_metadata.hops[0].offer_token;
        const tradeReply = page.waitForResponse(r => r.url().startsWith('https://indexer.dex.cl8y.com/api/v1/route/solve?') && new URL(r.url()).searchParams.get('token_in') === wrapped && new URL(r.url()).searchParams.get('amount_in') === '100000000' && r.status() === 200, {timeout: 15000});
        const query = new URLSearchParams({from: wrapped, to: q.token_out, amount: '100'});
        await page.goto('https://dex.cl8y.com/trade?' + query, {waitUntil: 'domcontentloaded', timeout: 15000});
        await page.getByTestId('trade-page-heading').waitFor({timeout: 10000});
        const trade = await (await tradeReply).json();
        check(trade.order_economics?.status === 'available', 'OriginalEvidenceUnavailable');
        await page.getByTestId('trade-market-expected-receive').waitFor({timeout: 10000});
        const td = trade.order_economics, tm = BigInt(td.order_movement), tb = BigInt(td.baseline_amount_out);
        const tf = BigInt(trade.spot_amount_out), tn = BigInt(trade.estimated_amount_out);
        const tdiff = tf > tn ? tf - tn : tn - tf, tdenom = tf > tn ? tf : tn;
        const tradeExpected = tm * 100n > tb * 5n || tdiff * 100n > tdenom * 10n;
        const tradeNoticeCount = await page.getByTestId('swap-impact-aside').count();
        check(tradeNoticeCount === Number(tradeExpected), 'LiveRiskNoticeMismatch');
        receipt.tradeMarket = {expectedRiskNotice: tradeExpected, riskNoticeCount: tradeNoticeCount};
        check(errors === 0, 'TradeRuntimeError');
        receipt.tradeSeriousOrCriticalAxe = await seriousViolations(page);
        check(receipt.tradeSeriousOrCriticalAxe === 0, 'LiveAccessibilityFailure');
        receipt.tradeNavigation = 'PASS';
      }
      await context.close();
    }
    receipt.overall = 'PASS';
  } catch (error) {
    // Error messages from page/transport can contain identifiers. Only our fixed
    // assertion codes enter receipts; otherwise keep the constructor name.
    const allowed = new Set(['OriginalEvidenceUnavailable','LiveRiskNoticeMismatch','LiveHorizontalOverflow','LiveCTAObscured','NativeRouteOrDisclosureMissing','LiveRuntimeError','TradeRuntimeError','LiveAccessibilityFailure','LiveFooterVisibility','LiveFormNotCentered','LiveRouteSpacing','LiveRedundantStatus','LiveReverseQuoteUnavailable','LiveQuoteScaleMismatch','LiveStatusDensity']);
    receipt.overall = 'FAIL';
    receipt.failure = allowed.has(error.message) ? error.message : error.constructor.name;
  } finally { await browser.close(); }
  fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n');
  console.log(JSON.stringify({overall: receipt.overall, failure: receipt.failure}));
  process.exitCode = receipt.overall === 'PASS' ? 0 : 1;
})().catch(() => { console.error('LiveProbeUnavailable'); process.exitCode = 1; });
