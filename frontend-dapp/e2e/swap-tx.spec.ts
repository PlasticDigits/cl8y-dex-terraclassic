import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertSwapCtaNotBlocked } from './helpers/chain'
import { clickSwapSubmit, swapActionPanel, readSwapYouReceiveAmount } from './helpers/swap-ui'
import { expectAtLeastTwoPayTokenOptions } from './helpers/token-select'
import { headerConnectedWalletButton } from './helpers/wallet-ui'

test.describe('Swap Transaction', () => {
  test('executes a swap with simulated wallet', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    // Fixture already opens `/`; avoid a full navigation here — it can clear dev-wallet session state.
    await expect(headerConnectedWalletButton(page)).toBeVisible({ timeout: 15000 })
    await page.waitForLoadState('networkidle')

    await expectAtLeastTwoPayTokenOptions(page)

    // Human-decimal amount (placeholder 0.00); keep small after bot-swarm / hybrid routes
    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.001')

    // Wait for simulation result
    await expect(async () => {
      const text = await readSwapYouReceiveAmount(page)
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapPanel = swapActionPanel(page)

    await expect(async () => {
      const calculating = swapPanel.getByRole('button', { name: /^Calculating/ })
      expect(await calculating.count()).toBe(0)
    }).toPass({ timeout: 120_000 })

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertSwapCtaNotBlocked(await swapAction.textContent())

    await clickSwapSubmit(page, swapPanel)

    await assertTxResultAlert(page)
  })
})
