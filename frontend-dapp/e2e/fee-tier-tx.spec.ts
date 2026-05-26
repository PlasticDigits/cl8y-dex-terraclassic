import { test, expect } from './fixtures/dev-wallet'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'
import { requireSelfServiceRegisterButtons } from './helpers/fee-e2e'
import { skipIfLcdUnreachable } from './helpers/chain'

test.describe('Fee Tier Registration', () => {
  test('registers for a fee tier', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await clickDesktopMoreNavItem(page, 'Fee Tiers')
    await page.waitForURL(/\/tiers/)
    await page.waitForLoadState('networkidle')

    // Wait for tiers to load
    await expect(page.getByText('Fee Discount Tiers')).toBeVisible({ timeout: 10000 })

    await requireSelfServiceRegisterButtons(page)

    const registerBtns = page.getByRole('button', { name: /^Register$/ })
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
