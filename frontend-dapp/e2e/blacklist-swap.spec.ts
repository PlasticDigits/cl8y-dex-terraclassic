import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import {
  factoryAddressFromEnv,
  routeTradingBlacklistCheck,
  walletBlacklistedLcdResponse,
  WALLET_BLACKLIST_ALERT_COPY,
} from './helpers/blacklist-lcd-mock'
import { swapActionPanel } from './helpers/swap-ui'
import { expectAtLeastTwoPayTokenOptions } from './helpers/token-select'
import { requireTokenInCombobox } from './helpers/wrap-e2e'

test.describe('Trading blacklist swap CTA (GitLab #388 / #422)', () => {
  test('mocked LCD wallet blacklist blocks Swap CTA', async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await routeTradingBlacklistCheck(page, factoryAddressFromEnv(), walletBlacklistedLcdResponse())
    await connectWallet
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)

    await requireTokenInCombobox(page, 'Select token you pay', 'LUNC', 'LUNC-C')
    await requireTokenInCombobox(page, 'Select token you receive', 'LUNC-C')

    await page.getByPlaceholder('0.00').first().fill('0.0001')

    const blacklistAlert = page.getByRole('alert').filter({ hasText: /protocol trading blacklist/i })
    await expect(blacklistAlert).toBeVisible({ timeout: 20_000 })
    await expect(blacklistAlert).toContainText(WALLET_BLACKLIST_ALERT_COPY)

    const btn = swapActionPanel(page).getByRole('button', { name: 'Trading restricted' })
    await expect(btn).toBeVisible({ timeout: 15_000 })
    await expect(btn).toBeDisabled()
  })
})
