import { expect, test } from '@playwright/test'
import { openFirstFactoryManage } from './helpers/pool-ui'

/** Playwright smoke for GitLab #660 (`pool-manage-660`). */
test.describe('Pool Manage IA (GitLab #660)', () => {
  test('T1: /pool has no page-level zap cards', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-lp-howto')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
    await expect(page.getByTestId('pool-one-sided-withdraw')).toHaveCount(0)
    await expect(page.getByTestId('pool-card-advanced')).toHaveCount(0)
  })

  test('T2: Manage expand shows four peer tabs, no form until a tab is selected', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page)
    await expect(page.getByTestId('pool-manage-tab-provide')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-withdraw')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-zap-add')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-zap-withdraw')).toBeVisible()
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
    await expect(page.getByTestId('pool-provide-field-label-a')).toHaveCount(0)
    await expect(page.getByTestId('pool-card-advanced')).toHaveCount(0)
  })

  test('T3: Provide Liquidity is two amounts, wrap checkboxes, IL, pre-sign', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page, 'provide')
    await expect(page.getByPlaceholder('0.00').first()).toBeVisible()
    await expect(page.getByText(/Asset A|Asset B/i)).toHaveCount(0)
    await expect(page.getByTestId('pool-il-risk-notice-advanced')).toBeVisible()
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
  })

  test('T4: Withdraw Liquidity is LP amount, not zap', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page, 'withdraw')
    await expect(page.getByLabel('LP Token Amount')).toBeVisible()
    await expect(page.getByTestId('pool-one-sided-withdraw')).toHaveCount(0)
  })

  test('T5: Zap Add is token + amount only (no pair picker)', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
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

  test('T6: Zap Withdraw pins this pair LP (no other-pair picker)', async ({ page }) => {
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page, 'zap-withdraw')
    const w = page.getByTestId('pool-one-sided-withdraw')
    await expect(w).toBeVisible()
    await expect(w.getByTestId('pool-one-sided-lp-pinned')).toBeVisible()
    await expect(w.getByLabel(/Withdraw as/i)).toBeVisible()
    await expect(w.getByTestId('pool-one-sided-withdraw-amount')).toBeVisible()
    await expect(w.getByText(/Receive as wrapped/i)).toHaveCount(0)
    await expect(w.getByRole('combobox', { name: /^LP$/i })).toHaveCount(0)
  })

  test('T13: 375px four tabs wrap and stay reachable', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 })
    await page.goto('/pool')
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await openFirstFactoryManage(page)
    const tabs = page.getByTestId('pool-manage-actions')
    await expect(tabs).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-provide')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-withdraw')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-zap-add')).toBeVisible()
    await expect(page.getByTestId('pool-manage-tab-zap-withdraw')).toBeVisible()
  })
})
