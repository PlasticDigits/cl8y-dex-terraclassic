import { test, expect } from './fixtures/dev-wallet'

test.describe.configure({ mode: 'serial' })

test.describe('Limit orders funded txs', () => {
  test('place limit shows success with tx hash', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 60_000 })

    await page.locator('#limit-pair').click()
    const firstOpt = page.getByRole('option').first()
    await expect(firstOpt).toBeVisible({ timeout: 15_000 })
    await firstOpt.click()

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    await placeCard.getByRole('button', { name: /^Place limit$/i }).click()

    await expect(placeCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })
    await expect(placeCard.locator('.alert-success')).toContainText(/TX:/i)
  })

  test('cancel limit submits after place (indexed order id)', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 60_000 })

    await page.locator('#limit-pair').click()
    await page.getByRole('option').first().click()

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    await placeCard.getByRole('button', { name: /^Place limit$/i }).click()
    await expect(placeCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })

    const idLocator = page.getByTestId('last-placed-order-id')
    await expect(idLocator).toBeVisible({ timeout: 45_000 })

    const cancelCard = page.locator('.card-neo').filter({ hasText: 'Cancel limit' })
    await cancelCard.getByRole('button', { name: /^Cancel limit$/i }).click()

    await expect(cancelCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })
    await expect(cancelCard.locator('.alert-success')).toContainText(/TX:/i)
  })
})
