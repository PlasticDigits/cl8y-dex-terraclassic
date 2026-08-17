import { expect, test } from '@playwright/test'

test.describe('One-sided pool add/withdraw UI (GitLab #533 P1–P3)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible({ timeout: 90_000 })
  })

  test('P1 add card: Token, Pair, Amount only — no wrap checkbox or second asset', async ({ page }) => {
    const add = page.getByTestId('pool-one-sided-add')
    await expect(add).toBeVisible()
    await expect(add.getByLabel(/^Token$/i)).toBeVisible()
    await expect(add.getByLabel(/^Pair$/i)).toBeVisible()
    await expect(add.getByTestId('pool-one-sided-add-amount')).toBeVisible()
    await expect(add.getByText(/auto-wrap/i)).toHaveCount(0)
    await expect(add.getByLabel(/Asset B/i)).toHaveCount(0)
    await expect(add.getByTestId('pool-il-risk-notice')).toBeVisible()
    await expect(add.getByTestId('pool-one-sided-add-submit')).toBeVisible()
  })

  test('P2 withdraw card: LP, Withdraw as, Amount — no receive-wrapped checkbox', async ({ page }) => {
    const w = page.getByTestId('pool-one-sided-withdraw')
    await expect(w).toBeVisible()
    await expect(w.getByLabel(/^LP$/i)).toBeVisible()
    await expect(w.getByLabel(/Withdraw as/i)).toBeVisible()
    await expect(w.getByTestId('pool-one-sided-withdraw-amount')).toBeVisible()
    await expect(w.getByText(/Receive as wrapped/i)).toHaveCount(0)
    await expect(w.getByTestId('pool-one-sided-withdraw-submit')).toBeVisible()
  })

  test('P3 native LUNC can be chosen as add-from without a counterpart field', async ({ page }) => {
    const add = page.getByTestId('pool-one-sided-add')
    await expect(add.getByTestId('pool-one-sided-add-amount')).toBeVisible()
    await expect(add.getByLabel(/Asset B amount/i)).toHaveCount(0)
    await expect(add.getByText(/Use native/i)).toHaveCount(0)
  })
})
