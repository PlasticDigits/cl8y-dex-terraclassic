import type { Page } from '@playwright/test'

/** Header wallet control when disconnected (matches WalletButton copy). */
export function headerConnectButton(page: Page) {
  return page.locator('header').getByRole('button', { name: /CONNECT TC|TC/i })
}

/** Header wallet control when connected (shortened address in button label). */
export function headerConnectedWalletButton(page: Page) {
  return page
    .locator('header')
    .getByRole('button')
    .filter({ hasText: /terra1/ })
}
