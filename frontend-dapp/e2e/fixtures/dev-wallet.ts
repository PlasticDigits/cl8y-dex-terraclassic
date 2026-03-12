import { test as base, expect } from '@playwright/test'

export const test = base.extend<{ connectWallet: void }>({
  connectWallet: [async ({ page }, use) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    const connectBtn = page.getByRole('button', { name: /CONNECT TC|TC/i }).first()
    await connectBtn.click()

    const simBtn = page.getByRole('button', { name: /Simulated Wallet/i })
    await expect(simBtn).toBeVisible({ timeout: 5000 })
    await simBtn.click()

    await expect(page.getByRole('button', { name: /terra1.*20k38v/ })).toBeVisible({ timeout: 5000 })

    await use()
  }, { auto: false }],
})

export { expect } from '@playwright/test'
