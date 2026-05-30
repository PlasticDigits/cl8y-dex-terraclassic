import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import {
  assertLimitPlaceCtaNotBlocked,
  fillValidLimitPrice,
  placeLimitCard,
  requireLimitTxPair,
  selectLimitPairByFactoryIndex,
  selectLimitSide,
  submitPlaceLimitAndExpectTx,
  submitLadderPlaceAndExpectTx,
  submitCancelLimitAndExpectTx,
  cancelLimitCard,
} from './helpers/limit-e2e'
import {
  fetchTxJson,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  txJsonHasWasmAction,
} from './helpers/lcd'

test.describe.configure({ mode: 'serial' })

test.describe('Limit orders funded txs', () => {
  test('place limit shows success with tx hash', async ({ page, connectWallet, request }) => {
    test.setTimeout(240_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    const { index } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, index)
    await selectLimitSide(page, 'ask')

    const placeCard = placeLimitCard(page)
    await fillValidLimitPrice(page, 'ask')
    await placeCard.getByPlaceholder('0.0').fill('0.001')
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeVisible({ timeout: 60_000 })
    assertLimitPlaceCtaNotBlocked(await placeBtn.textContent())
    await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
    await submitPlaceLimitAndExpectTx(page)
    const successAlert = placeCard.locator('.alert-success')

    const txHash = await readTxHashFromAlertLink(page, successAlert)
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'place_limit_order')).toBe(true)
    }).toPass({ timeout: 180_000 })
  })

  test('cancel limit submits after place (indexed order id)', async ({ page, connectWallet, request }) => {
    test.setTimeout(240_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    const { index } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, index)
    await selectLimitSide(page, 'ask')

    const placeCard = placeLimitCard(page)
    await fillValidLimitPrice(page, 'ask')
    await placeCard.getByPlaceholder('0.0').fill('0.001')
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
    await submitPlaceLimitAndExpectTx(page)

    const idLocator = page.getByTestId('last-placed-order-id')
    await expect(idLocator).toBeVisible({ timeout: 45_000 })

    const cancelCard = cancelLimitCard(page)
    const cancelBtn = cancelCard.getByRole('button', { name: /^Cancel limit$/i })
    await expect(cancelBtn).toBeEnabled({ timeout: 30_000 })
    await submitCancelLimitAndExpectTx(page)

    const cancelSuccess = cancelCard.locator('.alert-success')

    const cancelHash = await readTxHashFromAlertLink(page, cancelSuccess)
    await expect(async () => {
      const json = await fetchTxJson(request, cancelHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'cancel_limit_order')).toBe(true)
    }).toPass({ timeout: 180_000 })
  })

  test('place 5-rung ladder in one tx (batch hook)', async ({ page, connectWallet, request }) => {
    test.setTimeout(300_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    const { index } = await requireLimitTxPair(request, pairs)
    await selectLimitPairByFactoryIndex(page, index)

    await page.getByTestId('limit-place-mode-ladder').click()
    const ladderPanel = page.getByTestId('limit-order-ladder-panel')
    await expect(ladderPanel).toBeVisible({ timeout: 30_000 })

    await ladderPanel.getByTestId('ladder-start-price').fill('0.95')
    await ladderPanel.getByTestId('ladder-end-price').fill('1.05')
    await ladderPanel.getByTestId('ladder-rung-count').fill('5')
    await ladderPanel.getByTestId('ladder-total-amount').fill('100')

    await expect(ladderPanel.getByTestId('ladder-gas-summary')).toContainText(/saves/i, { timeout: 30_000 })
    await submitLadderPlaceAndExpectTx(page)

    const successAlert = ladderPanel.locator('.alert-success')

    const txHash = await readTxHashFromAlertLink(page, successAlert)
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'place_limit_order_batch')).toBe(true)
      expect(txJsonHasWasmAction(json, 'place_limit_order')).toBe(true)
    }).toPass({ timeout: 180_000 })
  })
})
