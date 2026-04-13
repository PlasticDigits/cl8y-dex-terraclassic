import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, skipIfNoTxAlert } from './helpers/chain'
import { swapYouReceiveAmountDisplay } from './helpers/swap-ui'
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

    // Enter a small swap amount (in micro units)
    const input = page.getByPlaceholder('0.00').first()
    await input.fill('1000000')

    // Wait for simulation result
    const youReceiveAmount = swapYouReceiveAmountDisplay(page)
    await expect(youReceiveAmount).not.toHaveText('0.00', { timeout: 15000 })

    // Primary swap card is the first shell panel in main (heading "Swap" is inside it)
    const swapPanel = page.locator('main .shell-panel-strong').first()

    await expect(async () => {
      const calculating = swapPanel.getByRole('button', { name: /^Calculating/ })
      expect(await calculating.count()).toBe(0)
    }).toPass({ timeout: 120_000 })

    if (
      await swapPanel
        .getByRole('button', { name: /^Insufficient Balance$/ })
        .isVisible()
        .catch(() => false)
    ) {
      test.skip(true, 'Dev wallet has no spendable balance for this token; fund the local dev account for swap-tx.')
    }
    if (
      await swapPanel
        .getByRole('button', { name: /^No Route$/ })
        .isVisible()
        .catch(() => false)
    ) {
      test.skip(true, 'No swap route for the current selection; ensure pools and router are deployed on local chain.')
    }

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    await expect(swapAction).toBeEnabled({ timeout: 30_000 })

    await swapAction.click()
    // High price impact (>5%) uses a two-step confirm; first click may only reveal "Confirm Swap (...)"
    await page.waitForTimeout(500)
    const confirmSwap = swapPanel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
    if (await confirmSwap.isVisible().catch(() => false)) {
      await confirmSwap.click()
    }

    await skipIfNoTxAlert(page)
  })
})
