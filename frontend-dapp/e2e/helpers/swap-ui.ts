import { expect, type Locator, type Page } from '@playwright/test'

import { assertSwapCtaNotBlocked } from './chain'

/** Matches the simulated output field in {@link SwapPage} (`swap-io-card-receive`). */
export function swapYouReceiveAmountDisplay(page: Page) {
  return page.locator('.swap-io-card-receive div.font-medium').first()
}

/** Primary swap panel in main (heading "Swap" is inside it). */
export function swapActionPanel(page: Page) {
  return page.locator('main .shell-panel-strong').first()
}

async function waitForSwapQuoteIdle(panel: Locator): Promise<void> {
  await expect(async () => {
    const calculating = panel.getByRole('button', { name: /^Calculating/ })
    expect(await calculating.count()).toBe(0)
  }).toPass({ timeout: 120_000 })
}

/** Native-wrap multihop routes often exceed 1% preset slippage on LocalTerra seed pools. */
export async function openSwapSettingsAndSetSlippage(page: Page, percent: number): Promise<void> {
  const panel = swapActionPanel(page)
  await panel.getByRole('button', { name: 'Settings' }).click()
  const settings = page.locator('#swap-slippage-settings')
  await expect(settings).toBeVisible()
  await settings.getByRole('textbox', { name: /Custom slippage protection/i }).fill(String(percent))
  await waitForSwapQuoteIdle(panel)
}

/** Expand Swap Settings → Advanced (integrator controls, GitLab #413). */
export async function expandSwapAdvancedSettings(page: Page): Promise<void> {
  const routeCheck = page.getByTestId('swap-indexer-route-check')
  if (!(await routeCheck.isVisible().catch(() => false))) {
    await page.getByTestId('swap-advanced-settings-toggle').click()
  }
  await expect(routeCheck).toBeVisible({ timeout: 15_000 })
}

/** Open Settings and expand Advanced for hybrid book leg / indexer route check. */
export async function openSwapAdvancedSettings(page: Page): Promise<void> {
  const panel = swapActionPanel(page)
  await panel.getByRole('button', { name: 'Settings' }).click()
  await expect(page.locator('#swap-slippage-settings')).toBeVisible()
  await expandSwapAdvancedSettings(page)
}

/** Click Swap / Confirm Swap (two-step when price impact is high). */
export async function clickSwapSubmit(page: Page, panel = swapActionPanel(page)) {
  await waitForSwapQuoteIdle(panel)

  const swapAction = panel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
  if ((await swapAction.count()) === 0) {
    const fallback = panel
      .getByRole('button')
      .filter({ hasText: /Swap|Confirm|Route|Connect|Insufficient|Rate Limit|Price impact|Enter Amount|Calculating/i })
      .first()
    assertSwapCtaNotBlocked(await fallback.textContent())
    expect(await swapAction.count(), 'expected Swap or Confirm Swap CTA').toBeGreaterThan(0)
  }

  await expect(swapAction).toBeEnabled({ timeout: 30_000 })
  assertSwapCtaNotBlocked(await swapAction.textContent())
  await swapAction.click()
  await page.waitForTimeout(500)
  const confirmSwap = panel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
  if (await confirmSwap.isVisible().catch(() => false)) {
    await confirmSwap.click()
  }
}
