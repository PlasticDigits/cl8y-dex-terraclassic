import { test, expect } from '@playwright/test'
import {
  clickPriceChartFullscreen,
  expectFullscreenButtonLabels,
  expectPriceChartCanvasMounted,
  expectTradeChartNotUnavailable,
  installPriceChartFullscreenMock,
  skipPriceChartStrictWhenChainOptional,
  waitForTradeChartToolbar,
} from './helpers/price-chart'

const TRADE_PAIR = process.env.E2E_TRADE_PAIR ?? 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

/**
 * Browser smoke for lightweight-charts canvas + fullscreen handler (GitLab #228).
 * Requires LocalTerra + indexer like other trade/chart E2E (strict CI `npm run test:e2e`).
 */
test.describe('Price chart Playwright smoke (GitLab #228)', () => {
  test.describe('/charts canvas', () => {
    test.skip(skipPriceChartStrictWhenChainOptional(), 'Requires indexer stack; unset PLAYWRIGHT_SKIP_CHAIN')

    test('mounts lightweight-charts canvas when indexer returns candles', async ({ page }) => {
      await page.goto('/charts', { waitUntil: 'domcontentloaded' })
      await expect(page.getByRole('heading', { name: /charts & analytics/i })).toBeVisible({
        timeout: 30_000,
      })

      await expectPriceChartCanvasMounted(page)
      await expect(page.getByTestId('charts-market-data-outage-banner')).toHaveCount(0)
    })

    test('mobile viewport shows canvas or loading without crash', async ({ page }) => {
      await page.setViewportSize({ width: 390, height: 844 })
      await page.goto('/charts', { waitUntil: 'domcontentloaded' })
      await expect(page.getByRole('heading', { name: /charts & analytics/i })).toBeVisible({
        timeout: 30_000,
      })

      const canvasRoot = page.getByTestId('price-chart-lightweight-canvas')
      const loading = page.getByText(/loading chart/i)
      await expect(async () => {
        const hasCanvas = await canvasRoot.isVisible().catch(() => false)
        const hasLoading = await loading.isVisible().catch(() => false)
        expect(hasCanvas || hasLoading).toBe(true)
      }).toPass({ timeout: 90_000 })

      if (await canvasRoot.isVisible()) {
        await expect(canvasRoot.locator('canvas').first()).toBeVisible()
      }
    })
  })

  test.describe('/trade chart (strict stack)', () => {
    test.skip(skipPriceChartStrictWhenChainOptional(), 'Requires indexer stack; unset PLAYWRIGHT_SKIP_CHAIN')

    test('shows canvas on trade page when market data is up', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto(`/trade/${TRADE_PAIR}`, { waitUntil: 'domcontentloaded' })

      await expect(page.getByRole('heading', { name: /^trade$/i })).toBeVisible({ timeout: 30_000 })
      await expectPriceChartCanvasMounted(page)
      await expectTradeChartNotUnavailable(page)
    })

    test('interval change keeps canvas mounted without outage panel', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto(`/trade/${TRADE_PAIR}`, { waitUntil: 'domcontentloaded' })
      await expectPriceChartCanvasMounted(page)

      const oneHour = page.getByRole('button', { name: '1h candle interval' })
      const oneDay = page.getByRole('button', { name: '1d candle interval' })
      await expect(oneHour).toBeVisible()
      await oneDay.click()

      await expectPriceChartCanvasMounted(page, 60_000)
      await expectTradeChartNotUnavailable(page)
    })

    test('chart loads without wallet connected', async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 })
      await page.goto('/trade', { waitUntil: 'domcontentloaded' })
      await expectPriceChartCanvasMounted(page)
    })
  })

  test.describe('fullscreen control', () => {
    test.skip(
      skipPriceChartStrictWhenChainOptional(),
      'Requires trade workspace + indexer; unset PLAYWRIGHT_SKIP_CHAIN'
    )

    test.beforeEach(async ({ page }) => {
      await installPriceChartFullscreenMock(page, 'resolve')
    })

    test('enter fullscreen updates aria-label via fullscreenchange', async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 })
      await page.goto('/trade', { waitUntil: 'domcontentloaded' })
      await waitForTradeChartToolbar(page)

      await expectFullscreenButtonLabels(page, false)
      await clickPriceChartFullscreen(page)
      await expectFullscreenButtonLabels(page, true)
    })

    test('exit fullscreen restores expand aria-label', async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 })
      await page.goto('/trade', { waitUntil: 'domcontentloaded' })
      await waitForTradeChartToolbar(page)

      await clickPriceChartFullscreen(page)
      await expectFullscreenButtonLabels(page, true)
      await clickPriceChartFullscreen(page)
      await expectFullscreenButtonLabels(page, false)
    })

    test('double fullscreen click leaves stable aria state', async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 })
      await page.goto('/trade', { waitUntil: 'domcontentloaded' })
      await waitForTradeChartToolbar(page)
      const btn = page.getByTestId('price-chart-fullscreen')

      await btn.click()
      await btn.click()
      await btn.click()
      await expectFullscreenButtonLabels(page, true)
    })
  })

  test.describe('fullscreen denied', () => {
    test.skip(
      skipPriceChartStrictWhenChainOptional(),
      'Requires trade workspace + indexer; unset PLAYWRIGHT_SKIP_CHAIN'
    )

    test.beforeEach(async ({ page }) => {
      await installPriceChartFullscreenMock(page, 'reject')
    })

    test('denied fullscreen does not throw; button remains', async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 })
      await page.goto('/trade', { waitUntil: 'domcontentloaded' })
      await waitForTradeChartToolbar(page)
      const btn = page.getByTestId('price-chart-fullscreen')

      await btn.click()
      await expect(btn).toBeVisible()
      await expectFullscreenButtonLabels(page, false)
    })
  })
})
