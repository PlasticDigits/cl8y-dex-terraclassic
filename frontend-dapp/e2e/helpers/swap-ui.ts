import type { Page } from '@playwright/test'

/** Matches the simulated output field in {@link SwapPage} (`swap-io-card-receive`). */
export function swapYouReceiveAmountDisplay(page: Page) {
  return page.locator('.swap-io-card-receive div.font-medium').first()
}
