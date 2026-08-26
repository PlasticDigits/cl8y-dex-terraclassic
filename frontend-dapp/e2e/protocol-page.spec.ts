import { test, expect } from './fixtures/dev-wallet'
import { factoryAddressFromEnv } from './helpers/blacklist-lcd-mock'

test.describe('Protocol page (GitLab #550 / #422)', () => {
  test('P1 stats card + oracle card + audit rows', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    const feePanel = page.getByTestId('protocol-fee-stats')
    if (await feePanel.count()) {
      const statsBox = page.getByTestId('protocol-global-stats')
      const hubBox = page.getByTestId('protocol-dex-hub-prices')
      const statsPos = await statsBox.boundingBox()
      const feePos = await feePanel.boundingBox()
      const hubPos = await hubBox.boundingBox()
      expect(statsPos && feePos && statsPos.y < feePos.y).toBeTruthy()
      expect(feePos && hubPos && feePos.y < hubPos.y).toBeTruthy()
      await expect(page.getByTestId('protocol-stat-fees-24h')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-7d')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-30d')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-24h-chg')).toBeVisible()
    }
    await expect(page.getByTestId('protocol-stat-liquidity')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-volume-24h')).toBeVisible()
    const dailyChart = page.getByTestId('protocol-volume-daily-chart')
    if (await dailyChart.count()) {
      await expect(dailyChart).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-daily')).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-hourly')).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-monthly')).toBeVisible()
    }
    await expect(page.getByTestId('protocol-dex-hub-prices')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-custc')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-dex-hub-lunc')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-ust1')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-ustr')).toBeVisible()
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
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-oracle-tab-ustc')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tab-lunc')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tab-vfdusd')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-lunc').click()
    await expect(page.getByRole('heading', { name: /LUNC \/ USD/i })).toBeVisible()
  })

  test('P3 one history table — no duplicate Recent USTC heading', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByRole('heading', { name: /Recent USTC\/USD history/i })).toHaveCount(0)
    await expect(page.getByTestId('protocol-oracle')).toHaveCount(1)
  })

  test('P4 tablet 820×1180 cards stack; tabs usable', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tabs')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-vfdusd').click()
    await expect(page.getByRole('heading', { name: /^vFDUSD$/i })).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-vfdusd-venus')).toBeVisible()
    await expect(page.getByRole('heading', { name: /1 vFDUSD Price/i })).toBeVisible()
    await expect(
      page.getByText('FDUSD reference price').or(page.getByText(/Failed to load oracle price/i))
    ).toBeVisible()
  })

  test('P5 phone 390×844 Protocol nav still works', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toBeVisible()
    const phoneFees = page.getByTestId('protocol-fee-stats')
    if (await phoneFees.count()) {
      await expect(phoneFees).toBeVisible()
      const feeBox = await phoneFees.boundingBox()
      const hubBox = await page.getByTestId('protocol-dex-hub-prices').boundingBox()
      expect(feeBox && hubBox && feeBox.y + feeBox.height <= hubBox.y + 8).toBeTruthy()
    }
    await expect(page.getByTestId('protocol-dex-hub-prices')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-lunc')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()
  })
})
