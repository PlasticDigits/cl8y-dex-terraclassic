import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertSwapCtaNotBlocked } from './helpers/chain'
import { clickSwapSubmit, swapActionPanel, swapYouReceiveAmountDisplay } from './helpers/swap-ui'
import { expectAtLeastTwoPayTokenOptions } from './helpers/token-select'
import { headerConnectedWalletButton } from './helpers/wallet-ui'

/** Delay LCD tx lookup so confirming-phase UI is observable on fast LocalTerra (GitLab #330). */
const CONFIRMING_POLL_DELAY_MS = 4_000

test.describe('Terra broadcast confirming TX link', () => {
  test('shows Confirming… button and in-flight TX link during poll', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)

    await page.route('**/cosmos/tx/v1beta1/txs/**', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, CONFIRMING_POLL_DELAY_MS))
      await route.continue()
    })

    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible({ timeout: 15000 })
    await page.waitForLoadState('networkidle')

    await expectAtLeastTwoPayTokenOptions(page)

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.001')

    const youReceiveAmount = swapYouReceiveAmountDisplay(page)
    await expect(youReceiveAmount).not.toHaveText('0.00', { timeout: 15000 })

    const swapPanel = swapActionPanel(page)

    await expect(async () => {
      const calculating = swapPanel.getByRole('button', { name: /^Calculating/ })
      expect(await calculating.count()).toBe(0)
    }).toPass({ timeout: 120_000 })

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertSwapCtaNotBlocked(await swapAction.textContent())

    await clickSwapSubmit(page, swapPanel)

    await expect(swapPanel.getByRole('button', { name: 'Confirming…' })).toBeVisible({ timeout: 30_000 })

    const pendingTx = page.getByTestId('terra-broadcast-pending-tx')
    await expect(pendingTx).toBeVisible({ timeout: 30_000 })
    await expect(page.getByText(/^TX:/)).toBeVisible()

    const title = await pendingTx.getAttribute('title')
    expect(title, 'pending link should carry full tx hash').toBeTruthy()
    expect(pendingTx).toHaveText(new RegExp(title!.slice(0, 8)))

    await assertTxResultAlert(page)
    await expect(pendingTx).toBeHidden({ timeout: 30_000 })
  })
})
