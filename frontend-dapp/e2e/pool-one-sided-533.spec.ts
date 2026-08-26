import { expect, test } from '@playwright/test'
import { openFirstFactoryManage } from './helpers/pool-ui'

test.describe('One-sided pool add/withdraw UI (GitLab #533 P1–P3 / #660)', () => {
  test('P1 zap add: Token + Amount only — no wrap checkbox, pair picker, or second asset', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
    await openFirstFactoryManage(page, 'zap-add')
    const add = page.getByTestId('pool-one-sided-add')
    await expect(add).toBeVisible()
    await expect(add.getByLabel(/^Token$/i)).toBeVisible()
    await expect(add.getByLabel(/^Pair$/i)).toHaveCount(0)
    await expect(add.getByTestId('pool-one-sided-add-amount')).toBeVisible()
    await expect(add.getByText(/auto-wrap/i)).toHaveCount(0)
    await expect(add.getByLabel(/Asset B/i)).toHaveCount(0)
    await expect(add.getByTestId('pool-il-risk-notice')).toBeVisible()
    await expect(add.getByTestId('pool-one-sided-add-submit')).toBeVisible()
  })

  test('P2 zap withdraw: this pair LP, Withdraw as, Amount — no receive-wrapped checkbox', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page, 'zap-withdraw')
    const w = page.getByTestId('pool-one-sided-withdraw')
    await expect(w).toBeVisible()
    await expect(w.getByTestId('pool-one-sided-lp-pinned')).toBeVisible()
    await expect(w.getByLabel(/Withdraw as/i)).toBeVisible()
    await expect(w.getByTestId('pool-one-sided-withdraw-amount')).toBeVisible()
    await expect(w.getByText(/Receive as wrapped/i)).toHaveCount(0)
    await expect(w.getByTestId('pool-one-sided-withdraw-submit')).toBeVisible()
  })

  test('P3 native LUNC can be chosen as add-from without a counterpart field', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page, 'zap-add')
    const add = page.getByTestId('pool-one-sided-add')
    await expect(add.getByTestId('pool-one-sided-add-amount')).toBeVisible()
    await expect(add.getByLabel(/Asset B amount/i)).toHaveCount(0)
    await expect(add.getByText(/Use native/i)).toHaveCount(0)
  })
})
