/**
 * GitLab #295 — limit ladder rung count input UX (no on-chain tx).
 * Run via: ./scripts/verify-issue-295-ladder-rung-ui.sh
 */
import { test, expect } from './fixtures/dev-wallet'
import { gotoAndCaptureFactoryPairsPage } from './helpers/lcd'
import { selectLimitPairByFactoryIndex } from './helpers/limit-e2e'

test.describe('Limit ladder rung count (#295)', () => {
  test.beforeEach(async ({ page }) => {
    // make dev on :5173 does not set VITE_PLAYWRIGHT_E2E; pre-seed risk ack (GitLab #138).
    await page.addInitScript(() => {
      window.localStorage.setItem('cl8y-dex-risk-ack', JSON.stringify({ v: 1 }))
    })
  })

  test('clear, type, over-max message, blur clamp', async ({ page, connectWallet }) => {
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    test.skip(pairs.length === 0, 'No factory pairs — run make deploy-local')

    await selectLimitPairByFactoryIndex(page, 0)

    await page.getByTestId('limit-place-mode-ladder').click()
    const ladderPanel = page.getByTestId('limit-order-ladder-panel')
    await expect(ladderPanel).toBeVisible({ timeout: 30_000 })

    const input = ladderPanel.getByTestId('ladder-rung-count')
    await expect(input).toHaveValue('5')

    await input.click()
    await input.fill('')
    await expect(input).toHaveValue('')
    await expect(ladderPanel.getByTestId('ladder-rung-count-error')).toHaveCount(0)

    await input.fill('3')
    await expect(input).toHaveValue('3')
    await expect(ladderPanel.getByTestId('ladder-rung-count-error')).toHaveCount(0)

    await input.fill('25')
    await expect(input).toHaveValue('25')
    await expect(ladderPanel.getByTestId('ladder-rung-count-error')).toContainText(/at most 20/i)

    await input.blur()
    await expect(input).toHaveValue('20')
    await expect(ladderPanel.getByTestId('ladder-rung-count-error')).toHaveCount(0)
  })
})
