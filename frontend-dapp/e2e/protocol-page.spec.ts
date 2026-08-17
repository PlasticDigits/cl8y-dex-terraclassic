import { test, expect } from './fixtures/dev-wallet'
import { factoryAddressFromEnv } from './helpers/blacklist-lcd-mock'

test.describe('Protocol page (GitLab #550 / #422)', () => {
  test('P1 stats card + oracle card + audit rows', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()

    let factory: string | null = null
    try {
      factory = factoryAddressFromEnv()
    } catch {
      factory = null
    }
    if (factory) {
      await expect(page.getByTestId('protocol-factory-address')).toContainText(factory)
      await expect(page.getByTestId('protocol-router-address')).toContainText(/^terra1/)
    } else {
      await expect(page.getByTestId('protocol-contract-addresses')).toContainText(/Factory|Not configured/i)
    }
  })

  test('P2 ticker chips update heading', async ({ page }) => {
    await page.goto('/protocol')
    await expect(page.getByTestId('protocol-oracle-tab-ustc')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-oracle-tab-lunc')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tab-vfdusd')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-lunc').click()
    await expect(page.getByRole('heading', { name: /LUNC \/ USD/i })).toBeVisible()
  })

  test('P3 one history table — no duplicate Recent USTC heading', async ({ page }) => {
    await page.goto('/protocol')
    await expect(page.getByTestId('protocol-oracle')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByRole('heading', { name: /Recent USTC\/USD history/i })).toHaveCount(0)
    await expect(page.getByTestId('protocol-oracle')).toHaveCount(1)
  })

  test('P4 tablet 820×1180 cards stack; tabs usable', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/protocol')
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-oracle-tabs')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-vfdusd').click()
    await expect(page.getByRole('heading', { name: /vFDUSD \/ USD/i })).toBeVisible()
  })

  test('P5 phone 390×844 Protocol nav still works', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/protocol')
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()
  })
})
