import { test, expect } from './fixtures/dev-wallet'

test.describe('Pool Transactions', () => {
  test('provides liquidity', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/pool')
    await page.waitForLoadState('networkidle')

    // Wait for pools to load
    await expect(async () => {
      const panels = await page.locator('.shell-panel-strong').count()
      expect(panels).toBeGreaterThan(0)
    }).toPass({ timeout: 20000 })

    // Click first "Provide Liquidity" button
    const provideBtn = page.getByRole('button', { name: 'Provide Liquidity' }).first()
    await provideBtn.click()

    // Fill amounts
    const inputs = page.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('1000000')
    await inputs.nth(1).fill('1000000')

    // Click Provide Liquidity submit button
    const submitBtn = page.getByRole('button', { name: /Provide Liquidity/i }).last()
    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    // Wait for result
    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })
})
