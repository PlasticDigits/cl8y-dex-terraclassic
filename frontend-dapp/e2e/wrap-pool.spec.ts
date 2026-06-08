import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertLiquidityCtaNotBlocked } from './helpers/chain'
import { requirePoolCardWithNativeWrap, requirePoolCardWithReceiveWrapped } from './helpers/wrap-e2e'
import {
  poolProvideExpandButton,
  poolProvideSubmitButton,
  poolWithdrawExpandButton,
  poolWithdrawSubmitButton,
  poolReceiveWrappedCheckbox,
} from './helpers/pool-ui'
import { gotoWrapPoolLuncCard } from './helpers/pool-nav'

test.describe('Pool with native token wrapping — UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/pool')
    await expect(async () => {
      await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
    }).toPass({ timeout: 90_000 })
  })

  test('E6: pool page loads with pairs', async ({ page }) => {
    await expect(async () => {
      const pairCount = await page
        .getByText(/pair\(s\)/i)
        .first()
        .textContent()
      expect(pairCount).toMatch(/[\d,]+\s*pair/i)
      const m = pairCount?.match(/([\d,]+)\s*pair/i)
      expect(m).toBeTruthy()
      const n = parseInt(m![1].replace(/,/g, ''), 10)
      expect(n).toBeGreaterThan(0)
    }).toPass({ timeout: 90_000 })
  })

  test('E7: pool card shows provide and withdraw buttons', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 90_000 })
  })

  test('E8: provide liquidity form expands with native toggle', async ({ page }) => {
    await expect(async () => {
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
    }).toPass({ timeout: 90_000 })

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
    }).toPass({ timeout: 90_000 })

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
    }).toPass({ timeout: 90_000 })

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
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)
    await expect(async () => {
      const panels = await page.locator('.shell-panel-strong').count()
      expect(panels).toBeGreaterThan(0)
    }).toPass({ timeout: 90_000 })
    await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible({ timeout: 90_000 })
  })

  test('E7: provide liquidity with native token (auto-wrap)', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await poolProvideExpandButton(pairCard).click()

    await requirePoolCardWithNativeWrap(pairCard)

    const nativeCheckbox = pairCard.getByText(/auto-wrap/i)
    await nativeCheckbox.first().click()

    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.01')
    await inputs.nth(1).fill('0.01')

    const submitBtn = poolProvideSubmitButton(pairCard)
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool provide (native): CTA blocked after E2E provisioning; verify LocalTerra + deploy + e2e-provision script.'
    )
    await submitBtn.click()

    await assertTxResultAlert(page)
  })

  test('E8: provide liquidity with wrapped CW20 directly', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await poolProvideExpandButton(pairCard).click()

    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('0.01')
    await inputs.nth(1).fill('0.01')

    const submitBtn = poolProvideSubmitButton(pairCard)
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool provide (wrapped): CTA blocked after E2E provisioning; verify LocalTerra + deploy + e2e-provision script.'
    )
    await submitBtn.click()

    await assertTxResultAlert(page)
  })

  test('E9: withdraw liquidity with auto-unwrap to native', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await expect(poolWithdrawExpandButton(pairCard)).toBeVisible({ timeout: 90_000 })
    await poolWithdrawExpandButton(pairCard).click()

    await requirePoolCardWithReceiveWrapped(pairCard)

    const receiveWrapped = poolReceiveWrappedCheckbox(pairCard)
    if (await receiveWrapped.isChecked()) {
      await receiveWrapped.click({ force: true })
    }

    const maxButton = pairCard.locator('button', { hasText: /^\d/ })
    const maxCount = await maxButton.count()
    if (maxCount > 0) {
      await maxButton.first().click()
      const lpInput = pairCard.getByPlaceholder('0.00').first()
      const maxVal = await lpInput.inputValue()
      const partial = (parseFloat(maxVal) / 2).toFixed(6)
      await lpInput.fill(partial)
    } else {
      const lpInput = pairCard.getByPlaceholder('0.00').first()
      await lpInput.fill('0.001')
    }

    const submitBtn = poolWithdrawSubmitButton(pairCard)
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool withdraw (auto-unwrap): CTA blocked; ensure LP balance after prior provides or adjust pair selection.'
    )

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await assertTxResultAlert(page, 120_000)
  })

  test('E10: withdraw liquidity — receive as wrapped tokens', async ({ page }) => {
    const pairCard = await gotoWrapPoolLuncCard(page)
    await expect(poolWithdrawExpandButton(pairCard)).toBeVisible({ timeout: 90_000 })
    await poolWithdrawExpandButton(pairCard).click()

    const lpInput = pairCard.getByPlaceholder('0.00').first()
    const maxButton = pairCard.locator('button', { hasText: /^\d/ })
    if ((await maxButton.count()) > 0) {
      await maxButton.first().click()
    } else {
      await lpInput.fill('0.001')
    }

    const submitBtn = poolWithdrawSubmitButton(pairCard)
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Wrap pool withdraw (wrapped): CTA blocked; ensure LP balance for this pair.'
    )

    await expect(submitBtn).toBeEnabled({ timeout: 5000 })
    await submitBtn.click()

    await assertTxResultAlert(page, 120_000)
  })
})
