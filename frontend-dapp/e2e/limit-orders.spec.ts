import { test, expect } from './fixtures/dev-wallet'

test.describe('Limit orders page', () => {
  test('shows Limits heading and pair selector', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.getByRole('heading', { name: 'Limit Orders' })).toBeVisible()
    await expect(page.getByText('Place limit')).toBeVisible()
    await expect(page.getByText('Cancel limit')).toBeVisible()
  })

  test('pair control is searchable combobox (listbox), not native select', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const pairControl = page.locator('#limit-pair')
    await expect(pairControl).toBeVisible()
    await expect(pairControl).toHaveAttribute('role', 'combobox')
    await expect(page.locator('select#limit-pair')).toHaveCount(0)
  })

  test('shows Connect Wallet on place when disconnected', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 30_000 })
    const placeCard = page.locator('.card-glass').filter({ hasText: 'Place limit' })
    await expect(placeCard.getByRole('button', { name: /Connect Wallet/i })).toBeVisible()
  })

  test('shows ladder create options when Ladder selected while disconnected (GitLab #494)', async ({ page }) => {
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const pairControl = page.locator('#limit-pair')
    await expect(pairControl).toBeVisible({ timeout: 30_000 })
    await pairControl.click()
    const option = page.getByRole('option').first()
    await expect(option).toBeVisible({ timeout: 15_000 })
    await option.click()

    const placeCard = page.getByTestId('limits-place-card')
    await placeCard.getByTestId('limit-place-mode-ladder').click()
    await expect(placeCard.getByTestId('limit-order-ladder-panel')).toBeVisible()
    await expect(placeCard.getByTestId('ladder-start-price')).toBeVisible()
    await expect(placeCard.getByTestId('ladder-end-price')).toBeVisible()
    await expect(placeCard.getByTestId('ladder-rung-count')).toBeVisible()
    await expect(placeCard.getByTestId('ladder-total-amount')).toBeVisible()
    await expect(placeCard.getByTestId('ladder-place-submit')).toHaveTextContent(/Connect Wallet/i)
  })
})
