import { test as base, expect } from '@playwright/test'

import { headerConnectButton, headerConnectedWalletButton } from '../helpers/wallet-ui'

export const test = base.extend<{ connectWallet: void }>({
  connectWallet: [
    async ({ page }, use) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const connected = headerConnectedWalletButton(page)
      if (!(await connected.isVisible().catch(() => false))) {
        const connectBtn = headerConnectButton(page)
        await connectBtn.click()

        const simBtn = page.getByRole('button', { name: /Simulated Wallet/i })
        await expect(simBtn).toBeVisible({ timeout: 5000 })
        await simBtn.click()
      }

      await expect(connected).toBeVisible({ timeout: 15_000 })

      await use()
    },
    { auto: false },
  ],
})

export { expect } from '@playwright/test'
