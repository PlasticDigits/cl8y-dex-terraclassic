import { test, expect } from './fixtures/dev-wallet'

/**
 * GitLab #616 M616-4 — option-2 listed-pair tax copy.
 * e2e-smoke (5 workers). Does not submit on-chain txs.
 *
 * Pair-direct vs multi-hop hint strings are unit-tested in
 * taxPreviewMaxSpend.test.ts (`usesRouter` true/false). This spec
 * proves Create chrome + Swap/Trade never ship stale option-1 copy.
 */
const OPTION1_SKIP = 'Route skips buy/sell tax'
const OPTION2_SCOPE = 'Buy/sell tax applies on every listed-pair swap.'

test.describe('Option-2 tax copy post-merge QA (GitLab #616)', () => {
  test('Create Token tax scope is every listed-pair swap (not pair-direct only)', async ({ page }) => {
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('create-token-tax-scope')).toContainText(OPTION2_SCOPE)
    await expect(page.locator('body')).not.toContainText(OPTION1_SKIP)
  })

  test('Swap page does not ship stale option-1 skip copy', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('body')).toBeVisible({ timeout: 20_000 })
    await expect(page.locator('body')).not.toContainText(OPTION1_SKIP)
  })

  test('Trade market page does not ship stale option-1 skip copy', async ({ page }) => {
    await page.goto('/trade')
    await expect(page.locator('body')).toBeVisible({ timeout: 20_000 })
    await expect(page.locator('body')).not.toContainText(OPTION1_SKIP)
  })
})
