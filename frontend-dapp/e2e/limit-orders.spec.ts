import { test, expect } from './fixtures/dev-wallet'

test.describe('Limit orders page', () => {
  test('shows Limits heading and pair selector', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.getByRole('heading', { name: 'Limit Orders' })).toBeVisible()
    await expect(page.getByText('Place limit')).toBeVisible()
    await expect(page.getByText('Cancel limit')).toBeVisible()
  })

  test('pair control is portaled MenuSelect (listbox), not native select', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const pairControl = page.locator('#limit-pair')
    await expect(pairControl).toBeVisible()
    await expect(pairControl).toHaveAttribute('aria-haspopup', 'listbox')
    await expect(page.locator('select#limit-pair')).toHaveCount(0)
  })

  test('shows Connect Wallet on place when disconnected', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 30_000 })
    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await expect(placeCard.getByRole('button', { name: /Connect Wallet/i })).toBeVisible()
  })
})
