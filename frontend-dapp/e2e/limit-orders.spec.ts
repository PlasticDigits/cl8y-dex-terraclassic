import { test, expect } from './fixtures/dev-wallet'

test.describe('Limit orders page', () => {
  test('shows Limits heading and pair selector', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.getByRole('heading', { name: 'Limit Orders' })).toBeVisible()
    await expect(page.getByText('Place limit')).toBeVisible()
    await expect(page.getByText('Cancel limit')).toBeVisible()
  })

  test('shows Connect Wallet on place when disconnected', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const btn = page.getByRole('button', { name: /Place limit/i }).last()
    await expect(btn).toBeVisible()
    await expect(btn).toHaveText(/Connect Wallet/i)
  })
})
