import { test, expect } from './fixtures/dev-wallet'

test.describe('Swap Transaction', () => {
  test('executes a swap with simulated wallet', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    // Wait for pairs to load in selector
    const pairSelector = page.locator('select[aria-label="Select trading pair"]')
    await expect(async () => {
      const options = await pairSelector.locator('option').count()
      expect(options).toBeGreaterThan(1)
    }).toPass({ timeout: 20000 })

    // Enter a small swap amount (in micro units)
    const input = page.getByPlaceholder('0.00').first()
    await input.fill('1000000')

    // Wait for simulation result
    const youReceiveAmount = page.locator('.card-neo').filter({ hasText: 'You Receive' }).locator('div.text-2xl')
    await expect(youReceiveAmount).not.toHaveText('0.00', { timeout: 15000 })

    // Click Swap
    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    // Wait for tx result
    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })
})
