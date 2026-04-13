import { test, expect } from './fixtures/dev-wallet'

test.describe.configure({ mode: 'serial' })

test.describe('Limit orders funded txs', () => {
  test('place limit shows success with tx hash', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const pairTrigger = page.locator('#limit-pair')
    await expect(pairTrigger).toBeVisible({ timeout: 60_000 })
    await expect(pairTrigger).toBeEnabled({ timeout: 60_000 })

    await pairTrigger.click()
    // pairInfosToMenuSelectOptions prepends { value: '', label: 'Select pair…' }; skip that row.
    const firstRealPair = page.getByRole('option').filter({ hasText: /\// }).first()
    await expect(firstRealPair).toBeVisible({ timeout: 15_000 })
    await firstRealPair.click()
    await expect(pairTrigger).toContainText(/\//, { timeout: 30_000 })

    const paused = page.getByRole('status').filter({ hasText: /paused by governance/i })
    if (await paused.isVisible().catch(() => false)) {
      test.skip(true, 'First factory pair is paused; pick another pair manually for local limit-order txs.')
    }

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
    await placeBtn.click()

    await expect(placeCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })
    await expect(placeCard.locator('.alert-success')).toContainText(/TX:/i)
  })

  test('cancel limit submits after place (indexed order id)', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/limits')
    await page.waitForLoadState('networkidle')
    const pairTrigger2 = page.locator('#limit-pair')
    await expect(pairTrigger2).toBeVisible({ timeout: 60_000 })
    await expect(pairTrigger2).toBeEnabled({ timeout: 60_000 })

    await pairTrigger2.click()
    await page.getByRole('option').filter({ hasText: /\// }).first().click()
    await expect(pairTrigger2).toContainText(/\//, { timeout: 30_000 })

    const paused2 = page.getByRole('status').filter({ hasText: /paused by governance/i })
    if (await paused2.isVisible().catch(() => false)) {
      test.skip(true, 'First factory pair is paused; pick another pair manually for local limit-order txs.')
    }

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    const placeBtn2 = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn2).toBeEnabled({ timeout: 60_000 })
    await placeBtn2.click()
    await expect(placeCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })

    const idLocator = page.getByTestId('last-placed-order-id')
    await expect(idLocator).toBeVisible({ timeout: 45_000 })

    const cancelCard = page.locator('.card-neo').filter({ hasText: 'Cancel limit' })
    await cancelCard.getByRole('button', { name: /^Cancel limit$/i }).click()

    await expect(cancelCard.locator('.alert-success')).toBeVisible({ timeout: 90_000 })
    await expect(cancelCard.locator('.alert-success')).toContainText(/TX:/i)
  })
})
