import { test, expect, type Page } from '@playwright/test'
import { expectPriceChartCanvasMounted } from './helpers/price-chart'
import {
  installScrollInvariantMarket,
  makeScrollCandles,
  SCROLL_PAIR_ADDRESS,
} from './helpers/scroll-invariants-fixture'

type ChartProbe = {
  chart: {
    timeScale(): {
      getVisibleLogicalRange(): { from: number; to: number } | null
      setVisibleLogicalRange(range: { from: number; to: number }): void
    }
  }
  setDataCalls: number
  updateCalls: number
  lastUpdateTime: number | null
}

declare global {
  interface Window {
    __scrollChart1316?: ChartProbe
    __scrollRefreshTimers1316?: { delay: number; scheduledAt: number; firedAt: number }[]
  }
}

/** Observe the real library through its Vite loader; no production test hooks or chart mocks. */
async function observeRealChart(page: Page): Promise<void> {
  await page.route('**/src/components/charts/priceChartLightweightModule.ts*', async (route) => {
    const response = await route.fetch()
    const original = await response.text()
    expect(original).toContain('export function loadPriceChartLightweightModule()')
    const body =
      original.replace('export function loadPriceChartLightweightModule()', 'function originalLightweightModule()') +
      `
export async function loadPriceChartLightweightModule() {
  const library = await originalLightweightModule();
  return {
    ...library,
    createChart(...args) {
      const chart = library.createChart(...args);
      const probe = { chart, setDataCalls: 0, updateCalls: 0, lastUpdateTime: null };
      window.__scrollChart1316 = probe;
      const addSeries = chart.addSeries.bind(chart);
      chart.addSeries = (...seriesArgs) => {
        const series = addSeries(...seriesArgs);
        const setData = series.setData.bind(series);
        const update = series.update.bind(series);
        series.setData = (...dataArgs) => {
          probe.setDataCalls++;
          return setData(...dataArgs);
        };
        series.update = (...dataArgs) => {
          probe.updateCalls++;
          if (seriesArgs[0].type === 'Candlestick') probe.lastUpdateTime = dataArgs[0].time;
          return update(...dataArgs);
        };
        return series;
      };
      return chart;
    },
  };
}
`
    await route.fulfill({ response, body, contentType: 'text/javascript' })
  })
}

for (const viewport of [
  { width: 1280, height: 720 },
  { width: 375, height: 667 },
]) {
  test(`same-interval 30s refresh preserves the panned range without rewriting history at ${viewport.width}×${viewport.height} (#1316)`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport)
    // Observe native polling callbacks rather than inferring timer duration from
    // request arrivals: lazy-load prefetching can precede the chart observer.
    await page.addInitScript(() => {
      const callbacks: { delay: number; scheduledAt: number; firedAt: number }[] = []
      window.__scrollRefreshTimers1316 = callbacks
      const realSetInterval = window.setInterval.bind(window)
      window.setInterval = (handler, timeout, ...args) => {
        if (timeout !== 30_000 || typeof handler !== 'function') {
          return realSetInterval(handler, timeout, ...args)
        }
        const scheduledAt = Date.now()
        return realSetInterval(
          (...callbackArgs: unknown[]) => {
            callbacks.push({ delay: timeout, scheduledAt, firedAt: Date.now() })
            Reflect.apply(handler, window, callbackArgs)
          },
          timeout,
          ...args
        )
      }
    })
    const browserErrors: string[] = []
    page.on('pageerror', (error) => browserErrors.push(error.message))
    const initialCandles = makeScrollCandles()
    let candleRequests = 0
    const candleRequestTimes: number[] = []
    let refreshEnabled = false
    const refreshedCandles = initialCandles.map((row, index) =>
      index === initialCandles.length - 1
        ? { ...row, close: '2.24', close_human: '2.24', high: '2.29', high_human: '2.29', volume_base: '3000000' }
        : row
    )
    await installScrollInvariantMarket(page, {
      candles: () => {
        candleRequests++
        candleRequestTimes.push(Date.now())
        return refreshEnabled ? refreshedCandles : initialCandles
      },
    })
    await observeRealChart(page)
    await page.goto(`/charts/${SCROLL_PAIR_ADDRESS}`, { waitUntil: 'domcontentloaded' })
    await expectPriceChartCanvasMounted(page)
    await expect
      .poll(() => page.evaluate(() => window.__scrollChart1316?.chart.timeScale().getVisibleLogicalRange()))
      .toMatchObject({ from: expect.any(Number), to: expect.any(Number) })
    await page.getByTestId('price-chart-lightweight-canvas').scrollIntoViewIfNeeded()

    const before = await page.evaluate(() => {
      const probe = window.__scrollChart1316!
      const fittedRange = probe.chart.timeScale().getVisibleLogicalRange()
      // Zoom into older candles, well away from the live edge. This is the real chart API.
      probe.chart.timeScale().setVisibleLogicalRange({ from: 20, to: 50 })
      return { fittedRange, setDataCalls: probe.setDataCalls, updateCalls: probe.updateCalls }
    })
    const chartBox = await page.getByTestId('price-chart-lightweight-canvas').boundingBox()
    expect(chartBox).not.toBeNull()
    const x = chartBox!.x + chartBox!.width / 2
    const y = chartBox!.y + 70
    await page.mouse.move(x, y)
    await page.mouse.down()
    await page.mouse.move(x + 80, y, { steps: 12 })
    await page.mouse.up()
    // Allow the chart's normal drag animation to settle before measuring preservation.
    await expect
      .poll(() => page.evaluate(() => window.__scrollChart1316!.chart.timeScale().getVisibleLogicalRange()!.to))
      .toBeLessThan(50)
    await page.waitForTimeout(1_000)
    const selectedRange = await page.evaluate(() =>
      window.__scrollChart1316!.chart.timeScale().getVisibleLogicalRange()
    )
    expect(selectedRange).not.toEqual(before.fittedRange)
    expect(selectedRange!.to - selectedRange!.from).toBeLessThan(35)

    const requestCountBeforeRefresh = candleRequests
    const timerCountBeforeRefresh = await page.evaluate(() => window.__scrollRefreshTimers1316!.length)
    refreshEnabled = true
    // Leave React Query's production 30,000ms polling interval intact: no clock advance or manual invalidation.
    await expect
      .poll(() => page.evaluate(() => window.__scrollRefreshTimers1316!.length), { timeout: 35_000, intervals: [250] })
      .toBeGreaterThan(timerCountBeforeRefresh)
    await expect
      .poll(() => candleRequests, { timeout: 35_000, intervals: [250] })
      .toBeGreaterThan(requestCountBeforeRefresh)
    await expect
      .poll(() => page.evaluate(() => window.__scrollChart1316!.updateCalls))
      .toBeGreaterThan(before.updateCalls)
    const after = await page.evaluate(() => {
      const probe = window.__scrollChart1316!
      return {
        range: probe.chart.timeScale().getVisibleLogicalRange(),
        setDataCalls: probe.setDataCalls,
        lastUpdateTime: probe.lastUpdateTime,
        pollingCallbacks: window.__scrollRefreshTimers1316,
      }
    })
    expect(after.setDataCalls).toBe(before.setDataCalls)
    expect(after.lastUpdateTime).toBe(Date.parse(initialCandles.at(-1)!.open_time) / 1000)
    expect(after.range!.from).toBeCloseTo(selectedRange!.from, 5)
    expect(after.range!.to).toBeCloseTo(selectedRange!.to, 5)
    expect(browserErrors).toEqual([])
    await test.info().attach('chart-refresh-range.json', {
      body: JSON.stringify({ viewport, selectedRange, after, candleRequests, candleRequestTimes }, null, 2),
      contentType: 'application/json',
    })
  })
}
