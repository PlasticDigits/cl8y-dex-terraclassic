import { test, expect } from './fixtures/dev-wallet'

/**
 * GitLab #602 P402-4 / P402-5 — Create Token retail chrome + /create copy-address only.
 * e2e-smoke (5 workers). Does not submit on-chain txs.
 */
const QUERY_A = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const QUERY_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

test.describe('Create Token post-merge QA (GitLab #602)', () => {
  test('P402-1: /token/create is Create Token, not the unavailable stub', async ({ page }) => {
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('create-token-unavailable')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: /create token/i })).toBeVisible()
    await expect(page.getByRole('heading', { name: /create pair/i })).toHaveCount(0)
  })

  test('P402-4: More menu lists Create Token (not Mint)', async ({ page }) => {
    await page.goto('/')
    await page
      .locator('header.app-header-shell nav.app-desktop-nav')
      .getByRole('button', { name: /^More$/i })
      .click()
    const createToken = page.getByRole('menuitem', { name: 'Create Token' })
    await expect(createToken).toBeVisible({ timeout: 15_000 })
    await expect(createToken).toHaveAttribute('href', '/token/create')
    await expect(page.getByRole('menuitem', { name: 'Mint' })).toHaveCount(0)
    await createToken.click()
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 15_000 })
  })

  test('P402-4: paid SKU shows PayWithAnyToken; free path has Create Token CTA when connected', async ({
    page,
    connectWallet,
  }) => {
    await connectWallet
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('create-token-name').fill('QaToken')
    await page.getByTestId('create-token-symbol').fill('QATK')
    await page.getByTestId('create-token-ack').check()
    await expect(page.getByTestId('create-token-free-cta')).toBeVisible()
    await page.getByTestId('create-token-sku-transfer_tax').check()
    await expect(page.getByTestId('create-token-pay')).toBeVisible()
    await expect(page.getByTestId('create-token-pay-copy')).toContainText('50 UST1')
  })

  test('P402-5: /create?a=&b= does not prefill Token A/B', async ({ page }) => {
    await page.goto(`/create?a=${QUERY_A}&b=${QUERY_B}`)
    await expect(page.getByRole('heading', { name: /create trading pair/i })).toBeVisible({
      timeout: 20_000,
    })
    await page.getByTestId('create-pair-custom-toggle-token-a').click()
    await page.getByTestId('create-pair-custom-toggle-token-b').click()
    await expect(page.getByTestId('create-pair-custom-address-token-a')).toHaveValue('')
    await expect(page.getByTestId('create-pair-custom-address-token-b')).toHaveValue('')
  })

  test('P402-4: /tokens catalog page is reachable when Create Token is configured', async ({ page }) => {
    await page.goto('/tokens')
    await expect(page.locator('body')).toBeVisible()
    await expect(page.getByText(/create token is not configured/i)).toHaveCount(0)
  })
})
