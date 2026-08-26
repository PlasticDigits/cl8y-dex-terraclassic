import { test, expect } from './fixtures/dev-wallet'

/**
 * Create Token / Migrate Token retail lead copy (#489 / #670).
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

  test('Migrate Token why headline + examples, no env-var essay', async ({ page }) => {
    await page.goto('/token/migrate')
    await expect(page.getByTestId('migrate-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByRole('heading', { name: /migrate token/i })).toBeVisible()
    const why = page.getByTestId('migrate-token-why')
    const examples = page.getByTestId('migrate-token-why-examples')
    await expect(why).toBeVisible()
    await expect(why).toHaveText(/Unlock 7 features for your token on CL8Y Dex by migrating today/i)
    await expect(examples).toBeVisible()
    await expect(examples).toContainText(/buy and sell tax/i)
    await expect(examples).toContainText('Auto liquidity')
    await expect(examples).toContainText('Launch guards')
    await expect(examples).not.toContainText('Minting')
    await expect(page.locator('body')).not.toContainText('VITE_COMMUNITY_MIGRATE_CODE_IDS')
    await expect(page.locator('body')).not.toContainText('allowlisted CW20')
    await expect(page.locator('body')).not.toContainText('50 UST1')
    await expect(page.locator('body')).not.toContainText('enable_feature')
  })

  test('phone 375×667: why copy readable, no horizontal overflow, no nested cards', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 })
    await page.goto('/token/migrate')
    await expect(page.getByTestId('migrate-token-why')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('migrate-token-why-examples')).toBeVisible()
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1
    )
    expect(overflow).toBe(false)
    await expect(page.locator('[data-testid="migrate-token-page"] .card-glass')).toHaveCount(0)
  })
})
