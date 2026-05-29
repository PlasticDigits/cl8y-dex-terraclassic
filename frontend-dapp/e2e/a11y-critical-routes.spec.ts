import type { Page } from '@playwright/test'
import { test, expect } from './fixtures/dev-wallet'
import { assertNoCriticalA11yViolations } from './helpers/a11y'
import { headerConnectButton, headerConnectedWalletButton } from './helpers/wallet-ui'

const CHART_CANVAS_EXCLUDE = ['[data-testid="price-chart-lightweight-canvas"] canvas'] as const

/** Route shell is always required; chart region when indexer/LCD is up (CI strict E2E). */
async function waitForTradeOrChartsShell(page: Page, heading: RegExp) {
  await expect(page.getByRole('heading', { name: heading })).toBeVisible({ timeout: 30_000 })
  const chartRegion = page.getByRole('region', { name: /price \(usd\)/i })
  const chartLoading = page.getByText(/loading chart/i)
  const chartOutage = page.getByTestId('trade-chart-unavailable')
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    if (await chartRegion.isVisible().catch(() => false)) return
    if (await chartLoading.isVisible().catch(() => false)) return
    if (await chartOutage.isVisible().catch(() => false)) return
    await page.waitForTimeout(250)
  }
  /* Indexer/LCD down in PLAYWRIGHT_SKIP_CHAIN=1 smoke — axe still runs on header + page chrome. */
}

/**
 * Accessibility CI gate for retail-critical surfaces (GitLab #214).
 * Runs in e2e-smoke with VITE_PLAYWRIGHT_E2E=true (risk modal suppressed).
 */
test.describe('Critical route accessibility (GitLab #214)', () => {
  test('trade page has no critical/serious axe violations', async ({ page }) => {
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await waitForTradeOrChartsShell(page, /^trade$/i)

    await assertNoCriticalA11yViolations(page, {
      exclude: [...CHART_CANVAS_EXCLUDE],
    })
  })

  test('charts page has no critical/serious axe violations', async ({ page }) => {
    await page.goto('/charts')
    await page.waitForLoadState('networkidle')
    await waitForTradeOrChartsShell(page, /charts & analytics/i)

    await assertNoCriticalA11yViolations(page, {
      exclude: [...CHART_CANVAS_EXCLUDE],
    })
  })

  test('wallet connect modal is accessible when disconnected', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await headerConnectButton(page).click()
    await expect(page.getByRole('dialog')).toBeVisible()
    await expect(page.getByRole('heading', { name: /connect wallet/i })).toBeVisible()

    await assertNoCriticalA11yViolations(page, {
      include: ['[role="dialog"]'],
    })
  })

  test('connected wallet menu is accessible', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    const trigger = headerConnectedWalletButton(page)
    await expect(trigger).toBeVisible()
    await trigger.click()
    await expect(page.getByRole('menu')).toBeVisible()

    await assertNoCriticalA11yViolations(page, {
      include: ['header'],
    })

    await page.keyboard.press('Escape')
    await expect(page.getByRole('menu')).toHaveCount(0)
    await expect(trigger).toBeFocused({ timeout: 5_000 })
  })
})
