import { test, expect } from './fixtures/dev-wallet'
import { assertTxResultAlert, skipIfLcdUnreachable } from './helpers/chain'
import {
  assertLimitPlaceCtaNotBlocked,
  requireLimitTxPair,
  selectLimitPairByFactoryIndex,
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

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeVisible({ timeout: 60_000 })
    assertLimitPlaceCtaNotBlocked(await placeBtn.textContent())
    await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
    await placeBtn.click()

    await assertTxResultAlert(page)
    const successAlert = placeCard.locator('.alert-success')
    await expect(successAlert).toContainText(/TX:/i)

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

    const placeCard = page.locator('.card-neo').filter({ hasText: 'Place limit' })
    await placeCard.getByPlaceholder('0.0').fill('1')
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
    await placeBtn.click()
    await assertTxResultAlert(page)

    const idLocator = page.getByTestId('last-placed-order-id')
    await expect(idLocator).toBeVisible({ timeout: 45_000 })

    const cancelCard = page.locator('.card-neo').filter({ hasText: 'Cancel limit' })
    await cancelCard.getByRole('button', { name: /^Cancel limit$/i }).click()

    const cancelSuccess = cancelCard.locator('.alert-success')
    await assertTxResultAlert(page)
    await expect(cancelSuccess).toContainText(/TX:/i)

    const cancelHash = await readTxHashFromAlertLink(page, cancelSuccess)
    await expect(async () => {
      const json = await fetchTxJson(request, cancelHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'cancel_limit_order')).toBe(true)
    }).toPass({ timeout: 180_000 })
  })
})
