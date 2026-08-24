import { test, expect } from './fixtures/dev-wallet'

/**
 * Create Token / Migrate Token retail lead copy (#489).
 * e2e-smoke (5 workers). Does not submit on-chain txs.
 */
test.describe('Create / Migrate Token lead copy', () => {
  test('Create Token keeps tax lead and links Migrate here', async ({ page }) => {
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByText(/tax is not the DEX swap fee/i)).toBeVisible()
    await expect(page.getByText(/CMM-only/i)).toBeVisible()
    await expect(page.getByRole('link', { name: 'Migrate here' })).toHaveAttribute('href', '/token/migrate')
    await expect(page.locator('body')).not.toContainText('VITE_COMMUNITY_MIGRATE_CODE_IDS')
    await expect(page.locator('body')).not.toContainText('6036')
  })

  test('Migrate Token lead is short and has no env-var essay', async ({ page }) => {
    await page.goto('/token/migrate')
    await expect(page.getByTestId('migrate-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByRole('heading', { name: /migrate token/i })).toBeVisible()
    await expect(page.getByText(/move an existing token onto this template/i)).toBeVisible()
    await expect(page.locator('body')).not.toContainText('VITE_COMMUNITY_MIGRATE_CODE_IDS')
    await expect(page.locator('body')).not.toContainText('allowlisted CW20')
  })
})
