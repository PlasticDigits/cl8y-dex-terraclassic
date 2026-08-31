import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert, assertSwapCtaNotBlocked } from './helpers/chain'
import { clickSwapSubmit, swapActionPanel, readSwapYouReceiveAmount } from './helpers/swap-ui'
import { expectAtLeastTwoPayTokenOptions } from './helpers/token-select'
import { headerConnectedWalletButton } from './helpers/wallet-ui'

/** Delay RPC tx lookup (`pollTx` uses JSON-RPC `abci_query`, not LCD REST — GitLab #305/#330). */
const CONFIRMING_POLL_DELAY_MS = 4_000

function isPollTxAbciQuery(postData: string | null): boolean {
  if (!postData) return false
  try {
    const body = JSON.parse(postData) as { method?: string; params?: { path?: string } }
    return body.method === 'abci_query' && (body.params?.path?.includes('GetTx') ?? false)
  } catch {
    return false
  }
}

test.describe('Terra broadcast confirming TX link', () => {
  test('shows Confirming… button and in-flight TX link during poll', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)

    await page.route('**/*', async (route) => {
      const request = route.request()
      if (request.method() === 'POST' && isPollTxAbciQuery(request.postData())) {
        await new Promise((resolve) => setTimeout(resolve, CONFIRMING_POLL_DELAY_MS))
      }
      await route.continue()
    })

    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible({ timeout: 15000 })
    await page.waitForLoadState('networkidle')

    await expectAtLeastTwoPayTokenOptions(page)

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.001')

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
