import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertLiquidityCtaNotBlocked } from './helpers/chain'

test.describe('Pool Transactions', () => {
  test('provides liquidity', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)

    await expect(async () => {
      const panels = await page.locator('.shell-panel-strong').count()
      expect(panels).toBeGreaterThan(0)
    }).toPass({ timeout: 90_000 })
    await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible({ timeout: 90_000 })

    const pairCard = page
      .locator('.shell-panel-strong')
      .filter({ hasText: /In router \(factory\)/ })
      .first()
    await pairCard.getByRole('button', { name: 'Provide Liquidity' }).click()

    // Fill amounts (human decimal strings; leave headroom vs wallet balances after globalSetup mint)
    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('10')
    await inputs.nth(1).fill('10')

    const submitBtn = pairCard.getByRole('button', { name: /^Provide Liquidity$/i })
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(
      await submitBtn.textContent(),
      'Provide liquidity CTA blocked after E2E provisioning; see docs/testing.md (pool-tx) and scripts/e2e-provision-dev-wallet.sh.'
    )
    await submitBtn.scrollIntoViewIfNeeded()
    await submitBtn.click()

    await assertTxResultAlert(page, 120_000)
  })
})
