import { expect, type Page } from '@playwright/test'
import { isChainOptional } from './chain'

/** Playwright `data-testid` for the lightweight-charts mount root. */
export const PRICE_CHART_CANVAS_TEST_ID = 'price-chart-lightweight-canvas'
export const PRICE_CHART_FULLSCREEN_TEST_ID = 'price-chart-fullscreen'
export const TRADE_CHART_UNAVAILABLE_TEST_ID = 'trade-chart-unavailable'

const EXPAND_LABEL = 'Expand chart to fullscreen'
const EXIT_LABEL = 'Exit chart fullscreen'

/**
 * Strict chart canvas checks need indexer + deploy (same stack as trade layout E2E).
 * UI-only smoke (`PLAYWRIGHT_SKIP_CHAIN=1`) skips these tests.
 */
export function skipPriceChartStrictWhenChainOptional(): boolean {
  return isChainOptional()
}

/**
 * Wait until lightweight-charts mounted: test id visible and at least one child canvas.
 * Accepts loading → canvas transition; fails if trade outage panel appears.
 */
export async function expectPriceChartCanvasMounted(page: Page, timeoutMs = 90_000): Promise<void> {
  const outage = page.getByTestId(TRADE_CHART_UNAVAILABLE_TEST_ID)
  const canvasRoot = page.getByTestId(PRICE_CHART_CANVAS_TEST_ID)

  await expect(async () => {
    if (await outage.isVisible().catch(() => false)) {
      throw new Error('trade-chart-unavailable visible — indexer/LCD likely down')
    }
    await expect(canvasRoot).toBeVisible()
    await expect(canvasRoot.locator('canvas').first()).toBeVisible()
  }).toPass({ timeout: timeoutMs })
}

export async function expectTradeChartNotUnavailable(page: Page): Promise<void> {
  await expect(page.getByTestId(TRADE_CHART_UNAVAILABLE_TEST_ID)).toHaveCount(0)
}

/** Mock Fullscreen API so aria-label toggles via `fullscreenchange` (GitLab #228, #113). */
export async function installPriceChartFullscreenMock(
  page: Page,
  mode: 'resolve' | 'reject' = 'resolve'
): Promise<void> {
  await page.addInitScript((rejectEnter: boolean) => {
    let fsEl: Element | null = null
    Object.defineProperty(document, 'fullscreenElement', {
      configurable: true,
      get() {
        return fsEl
      },
    })
    Element.prototype.requestFullscreen = async function requestFullscreenMock() {
      if (rejectEnter) {
        throw new DOMException('Fullscreen denied', 'NotAllowedError')
      }
      fsEl = this as Element
      document.dispatchEvent(new Event('fullscreenchange'))
    }
    document.exitFullscreen = async function exitFullscreenMock() {
      fsEl = null
      document.dispatchEvent(new Event('fullscreenchange'))
    }
  }, mode === 'reject')
}

export async function expectFullscreenButtonLabels(page: Page, expanded: boolean): Promise<void> {
  const btn = page.getByTestId(PRICE_CHART_FULLSCREEN_TEST_ID)
  await expect(btn).toHaveAttribute('aria-label', expanded ? EXIT_LABEL : EXPAND_LABEL)
  await expect(btn).toHaveAttribute('aria-pressed', expanded ? 'true' : 'false')
}

export async function clickPriceChartFullscreen(page: Page): Promise<void> {
  await page.getByTestId(PRICE_CHART_FULLSCREEN_TEST_ID).click()
}

/** Trade workspace must resolve a pair before `PriceChart` toolbar mounts (GitLab #180). */
export async function waitForTradeChartToolbar(page: Page, timeoutMs = 90_000): Promise<void> {
  await expect(async () => {
    await expect(page.getByRole('heading', { name: /^trade$/i })).toBeVisible()
    await expect(page.getByTestId(PRICE_CHART_FULLSCREEN_TEST_ID)).toBeVisible()
  }).toPass({ timeout: timeoutMs })
}
