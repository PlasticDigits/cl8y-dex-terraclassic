import { test, expect } from './fixtures/dev-wallet'
import { factoryAddressFromEnv } from './helpers/blacklist-lcd-mock'

test.describe('Protocol page smoke (GitLab #422)', () => {
  test('renders factory and router contract addresses', async ({ page }) => {
    const factory = factoryAddressFromEnv()
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /protocol & oracle/i })).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()

    const factoryRow = page.getByTestId('protocol-factory-address')
    await expect(factoryRow).toBeVisible()
    await expect(factoryRow).toContainText(factory)

    const routerRow = page.getByTestId('protocol-router-address')
    await expect(routerRow).toBeVisible()
    await expect(routerRow).toContainText(/^terra1/)
  })
})
