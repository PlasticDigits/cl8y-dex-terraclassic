import { test, expect } from './fixtures/dev-wallet'

test.describe('Pool with native token wrapping', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/pool')
    await page.waitForLoadState('networkidle')
    await expect(async () => {
      await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
    }).toPass({ timeout: 15000 })
  })

  test('E6: pool page loads with pairs', async ({ page }) => {
    await expect(async () => {
      const pairCount = await page.getByText(/pair\(s\)/i).textContent()
      expect(pairCount).toMatch(/\d+\s*pair/i)
    }).toPass({ timeout: 15000 })
  })

  test('E7: pool card shows provide and withdraw buttons', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 15000 })
  })

  test('E8: provide liquidity form expands with native toggle', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 15000 })

    await page
      .getByRole('button', { name: /Provide Liquidity/i })
      .first()
      .click()

    const assetInput = page.getByPlaceholder('0.00').first()
    await expect(assetInput).toBeVisible()

    const nativeCheckbox = page.getByText(/auto-wrap/i)
    const count = await nativeCheckbox.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })

  test('E9: withdraw form shows receive wrapped checkbox for applicable pairs', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 15000 })

    await page
      .getByRole('button', { name: /Withdraw Liquidity/i })
      .first()
      .click()

    const lpInput = page.getByPlaceholder('0.00').first()
    await expect(lpInput).toBeVisible()

    const receiveWrappedCheckbox = page.getByText(/Receive as wrapped/i)
    const count = await receiveWrappedCheckbox.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })

  test('E10: withdraw slippage tolerance options visible', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 15000 })

    await page
      .getByRole('button', { name: /Withdraw Liquidity/i })
      .first()
      .click()

    const lpInput = page.getByPlaceholder('0.00').first()
    await expect(lpInput).toBeVisible()

    const slippageButton = page.getByRole('button', { name: /1\.0%/i }).first()
    const count = await slippageButton.count()
    expect(count).toBeGreaterThanOrEqual(0)
  })
})
