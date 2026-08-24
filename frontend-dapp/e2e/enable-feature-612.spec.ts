import { test, expect } from './fixtures/dev-wallet'

/**
 * GitLab #612 M612-6 — Enable Feature chrome (launcher payee, Minting omitted).
 * e2e-smoke (5 workers). Does not submit on-chain txs.
 */
test.describe('Enable Feature post-merge QA (GitLab #612)', () => {
  test('create paid SKU shows 50 UST1 PayWithAnyToken (not a token-direct hook)', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('create-token-name').fill('EnableFeat')
    await page.getByTestId('create-token-symbol').fill('ENFT')
    await page.getByTestId('create-token-ack').check()
    await page.getByTestId('create-token-sku-transfer_tax').check()
    await expect(page.getByTestId('create-token-pay')).toBeVisible()
    await expect(page.getByTestId('create-token-pay-copy')).toContainText('50 UST1')
    await page.getByTestId('create-token-sku-variable_rates').check()
    await expect(page.getByTestId('create-token-pay-copy')).toContainText('100 UST1')
  })

  test('Manage Token rejects a non-bech32 address before Enable Feature chrome', async ({ page }) => {
    await page.goto('/token/terra1invalid612manage/manage')
    await expect(page.getByTestId('manage-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('manage-token-invalid')).toBeVisible()
    await expect(page.getByTestId('manage-enable-feature')).toHaveCount(0)
    await expect(page.getByTestId('manage-unlock-sku')).toHaveCount(0)
  })

  test('Create Token SKU list still includes Minting (create-only)', async ({ page }) => {
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('create-token-sku-mint_control')).toBeVisible()
    await expect(page.getByTestId('create-token-sku-transfer_tax')).toBeVisible()
    await expect(page.getByTestId('create-token-sku-variable_rates')).toBeVisible()
  })
})
