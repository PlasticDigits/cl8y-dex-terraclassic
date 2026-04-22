import { test, expect } from './fixtures/dev-wallet'
import {
  skipIfLcdUnreachable,
  skipIfNoTxAlert,
  assertTxResultAlert,
  assertLiquidityCtaNotBlocked,
  isLocalTerraOptional,
} from './helpers/chain'

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

    // Click first "Provide Liquidity" button
    const provideBtn = page.getByRole('button', { name: 'Provide Liquidity' }).first()
    await provideBtn.click()

    // Fill amounts (human decimal strings; leave headroom vs wallet balances after globalSetup mint)
    const inputs = page.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill('10')
    await inputs.nth(1).fill('10')

    // Click Provide Liquidity submit button
    const submitBtn = page.getByRole('button', { name: /Provide Liquidity/i }).last()
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    const submitLabel = await submitBtn.textContent()
    if (isLocalTerraOptional()) {
      if (submitLabel?.includes('Insufficient') || submitLabel?.includes('Connect')) {
        test.skip(true, 'Provide liquidity CTA blocked; fund the dev wallet or run with LocalTerra + deploy.')
      }
    } else {
      assertLiquidityCtaNotBlocked(
        submitLabel,
        'Provide liquidity CTA blocked after E2E provisioning; see docs/testing.md (pool-tx) and scripts/e2e-provision-dev-wallet.sh.'
      )
    }
    await submitBtn.click()

    if (isLocalTerraOptional()) {
      await skipIfNoTxAlert(page)
    } else {
      await assertTxResultAlert(page)
    }
  })
})
