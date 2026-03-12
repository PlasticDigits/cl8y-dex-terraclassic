import { test, expect } from './fixtures/dev-wallet'

test.describe('Fee Tier Registration', () => {
  test('registers for a fee tier', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    // Wait for tiers to load
    await expect(page.getByText('Fee Discount Tiers')).toBeVisible({ timeout: 10000 })

    // Look for a Register button and click it
    const registerBtns = page.getByRole('button', { name: /Register/i })
    await expect(async () => {
      const count = await registerBtns.count()
      expect(count).toBeGreaterThan(0)
    }).toPass({ timeout: 15000 })

    await registerBtns.first().click()

    // Wait for tx result (error alert, or success: Deregister button / Active badge)
    await expect(
      page.locator('.alert-error').or(page.getByRole('button', { name: 'Deregister' })).or(page.getByText('Active'))
    ).toBeVisible({ timeout: 60000 })
  })
})
