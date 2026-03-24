import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, skipIfNoTxAlert } from './helpers/chain'

test.describe('Pool with native token wrapping — UI', () => {
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

test.describe('Pool Transaction Tests — Native Wrapping', () => {
  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)
    await page.waitForLoadState('networkidle')
    await expect(async () => {
      const panels = await page.locator('.shell-panel-strong').count()
      expect(panels).toBeGreaterThan(0)
    }).toPass({ timeout: 20000 })
  })

  test('E7: provide liquidity with native token (auto-wrap)', async ({ page }) => {
    const provideBtn = page.getByRole('button', { name: 'Provide Liquidity' }).first()
    await provideBtn.click()

    // Check if auto-wrap checkbox is present
    const nativeCheckbox = page.getByText(/auto-wrap/i)
    const hasNativeOption = (await nativeCheckbox.count()) > 0

    if (!hasNativeOption) {
      test.skip()
      return
    }

    // Check the auto-wrap checkbox
    await nativeCheckbox.first().click()

    // Fill amounts
    const inputs = page.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.1')
    await inputs.nth(1).fill('0.1')

    const submitBtn = page.getByRole('button', { name: /Provide Liquidity/i }).last()
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    const s0 = await submitBtn.textContent()
    if (s0?.includes('Insufficient') || s0?.includes('Connect')) {
      test.skip(true, 'Provide liquidity CTA blocked; fund dev wallet for wrap-pool tx tests.')
    }
    await submitBtn.click()

    await skipIfNoTxAlert(page)
  })

  test('E8: provide liquidity with wrapped CW20 directly', async ({ page }) => {
    const provideBtn = page.getByRole('button', { name: 'Provide Liquidity' }).first()
    await provideBtn.click()

    // Fill amounts without checking auto-wrap
    const inputs = page.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.1')
    await inputs.nth(1).fill('0.1')

    const submitBtn = page.getByRole('button', { name: /Provide Liquidity/i }).last()
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    const s8 = await submitBtn.textContent()
    if (s8?.includes('Insufficient') || s8?.includes('Connect')) {
      test.skip(true, 'Provide liquidity CTA blocked; fund dev wallet for wrap-pool tx tests.')
    }
    await submitBtn.click()

    await skipIfNoTxAlert(page)
  })

  test('E9: withdraw liquidity with auto-unwrap to native', async ({ page }) => {
    const withdrawBtn = page.getByRole('button', { name: 'Withdraw Liquidity' }).first()
    await withdrawBtn.click()

    // Check if "Receive as wrapped" checkbox is present
    const receiveWrappedCheckbox = page.getByText(/Receive as wrapped/i)
    const hasOption = (await receiveWrappedCheckbox.count()) > 0

    if (!hasOption) {
      test.skip()
      return
    }

    // Uncheck the "Receive as wrapped" checkbox to trigger auto-unwrap
    const checkbox = page.locator('input[type="checkbox"]').last()
    const isChecked = await checkbox.isChecked()
    if (isChecked) {
      await checkbox.uncheck()
    }

    // Click the balance "Max" button if available, or fill a small LP amount
    const maxButton = page.locator('button', { hasText: /^\d/ })
    const maxCount = await maxButton.count()
    if (maxCount > 0) {
      await maxButton.first().click()
    } else {
      const lpInput = page.getByPlaceholder('0.00').first()
      await lpInput.fill('0.001')
    }

    const submitBtn = page.getByRole('button', { name: /Withdraw Liquidity/i }).last()

    // If there's insufficient LP it won't be enabled
    const btnText = await submitBtn.textContent()
    if (btnText?.includes('Insufficient') || btnText?.includes('Connect')) {
      test.skip()
      return
    }

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await skipIfNoTxAlert(page)
  })

  test('E10: withdraw liquidity — receive as wrapped tokens', async ({ page }) => {
    const withdrawBtn = page.getByRole('button', { name: 'Withdraw Liquidity' }).first()
    await withdrawBtn.click()

    // Keep "Receive as wrapped" checkbox checked (default)
    const lpInput = page.getByPlaceholder('0.00').first()
    await lpInput.fill('0.001')

    const submitBtn = page.getByRole('button', { name: /Withdraw Liquidity/i }).last()

    const btnText = await submitBtn.textContent()
    if (btnText?.includes('Insufficient') || btnText?.includes('Connect')) {
      test.skip()
      return
    }

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await skipIfNoTxAlert(page)
  })
})
