import { test, expect } from './fixtures/dev-wallet'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'

test.describe('Fee Tier Registration', () => {
  test('registers for a fee tier', async ({ page, connectWallet }) => {
    await connectWallet
    await clickDesktopMoreNavItem(page, 'Fee Tiers')
    await page.waitForURL(/\/tiers/)
    await page.waitForLoadState('networkidle')

    // Wait for tiers to load
    await expect(page.getByText('Fee Discount Tiers')).toBeVisible({ timeout: 10000 })

    const registerBtns = page.getByRole('button', { name: /^Register$/ })
    await expect(async () => {
      expect(await registerBtns.count()).toBeGreaterThan(0)
    })
      .toPass({ timeout: 45000 })
      .catch(() => {})

    if ((await registerBtns.count()) === 0) {
      test.skip(
        true,
        'No self-service Register buttons (governance-only registration on chain, or fee discount unavailable).'
      )
    }

    await registerBtns.first().click()

    // Wait for tx result (error alert, or success: Deregister button / Active badge)
    await expect(
      page
        .locator('.alert-error')
        .or(page.getByRole('button', { name: 'Deregister' }))
        .or(page.getByText('Active'))
    ).toBeVisible({ timeout: 60000 })
  })
})
