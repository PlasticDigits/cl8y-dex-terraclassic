import { expect, type Page } from '@playwright/test'

/** Matches the simulated output field in {@link SwapPage} (`swap-io-card-receive`). */
export function swapYouReceiveAmountDisplay(page: Page) {
  return page.locator('.swap-io-card-receive div.font-medium').first()
}

/** Primary swap panel in main (heading "Swap" is inside it). */
export function swapActionPanel(page: Page) {
  return page.locator('main .shell-panel-strong').first()
}

/** Click Swap / Confirm Swap (two-step when price impact is high). */
export async function clickSwapSubmit(page: Page, panel = swapActionPanel(page)) {
  const swapAction = panel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
  await expect(swapAction).toBeEnabled({ timeout: 30_000 })
  await swapAction.click()
  await page.waitForTimeout(500)
  const confirmSwap = panel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
  if (await confirmSwap.isVisible().catch(() => false)) {
    await confirmSwap.click()
  }
}
