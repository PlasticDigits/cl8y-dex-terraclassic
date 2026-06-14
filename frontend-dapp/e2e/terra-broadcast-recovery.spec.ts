import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertSwapCtaNotBlocked } from './helpers/chain'
import {
  clickSwapSubmit,
  openSwapSettingsAndSetSlippage,
  swapActionPanel,
  swapYouReceiveAmountDisplay,
} from './helpers/swap-ui'
import { expectAtLeastTwoPayTokenOptions } from './helpers/token-select'
import { headerConnectedWalletButton } from './helpers/wallet-ui'

const BROADCAST_HANG_MS = 5_000

function isBroadcastTxSync(postData: string | null): boolean {
  if (!postData) return false
  try {
    const body = JSON.parse(postData) as { method?: string }
    return body.method === 'broadcast_tx_sync'
  } catch {
    return false
  }
}

test.describe('Terra broadcast post-sign recovery (GitLab #368)', () => {
  test('Simulated Wallet swap shows recovery UX when broadcast RPC hangs then confirms', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)

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

    if (
      await page
        .getByTestId('swap-slippage-blocked')
        .isVisible()
        .catch(() => false)
    ) {
      await page.getByTestId('swap-enable-expert-mode').click()
      await page.getByTestId('expert-mode-confirm-input').fill('ENABLE EXPERT MODE')
      await page.getByTestId('expert-mode-confirm-enable').click()
      await openSwapSettingsAndSetSlippage(page, 50)
    }

    const swapAction = swapPanel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
    await expect(swapAction).toBeVisible({ timeout: 60_000 })
    assertSwapCtaNotBlocked(await swapAction.textContent())

    let releaseBroadcast: (() => void) | null = null
    const broadcastGate = new Promise<void>((resolve) => {
      releaseBroadcast = resolve
    })

    await page.route('**/*', async (route) => {
      const req = route.request()
      if (req.method() === 'POST' && isBroadcastTxSync(req.postData())) {
        await broadcastGate
      }
      await route.continue()
    })

    await clickSwapSubmit(page, swapPanel)

    await expect(swapPanel.getByRole('button', { name: 'Checking broadcast…' })).toBeVisible({
      timeout: 45_000,
    })
    await expect(page.getByTestId('terra-broadcast-recovery-status')).toContainText(/broadcast status unknown/i)
    await expect(swapPanel.getByRole('button', { name: 'Checking broadcast…' })).toBeDisabled()

    const pendingTx = page.getByTestId('terra-broadcast-pending-tx')
    await expect(pendingTx).toBeVisible({ timeout: 10_000 })

    setTimeout(() => releaseBroadcast?.(), BROADCAST_HANG_MS)

    await assertTxResultAlert(page)
    await expect(pendingTx).toBeHidden({ timeout: 90_000 })
  })
})
