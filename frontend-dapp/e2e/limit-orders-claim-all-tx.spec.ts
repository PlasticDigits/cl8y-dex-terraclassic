import { test, expect } from './fixtures/dev-wallet'
import { assertTxResultAlert, skipIfLcdUnreachable } from './helpers/chain'
import {
  DEV_WALLET,
  fetchParkedExpiredPlacementsForDevWallet,
  firstDualCwPairAddr,
  indexerBaseUrl,
  seedExpiredParkedLimitsForClaimAllE2e,
} from './helpers/limit-expiry-park-e2e'
import { requireLimitTxPair, selectLimitPairByFactoryIndex } from './helpers/limit-e2e'
import {
  fetchTxJson,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  txJsonHasWasmAction,
} from './helpers/lcd'

test.describe.configure({ mode: 'serial' })

test.describe('Claim all parked limit refunds (GitLab #259)', () => {
  test('expiry-park harness + claim all parked batch tx', async ({ page, connectWallet, request }) => {
    test.setTimeout(420_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    seedExpiredParkedLimitsForClaimAllE2e()
    const pairAddr = await firstDualCwPairAddr(request)
    const parked = await fetchParkedExpiredPlacementsForDevWallet(request, pairAddr, 2)
    expect(parked.length).toBeGreaterThanOrEqual(2)

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
    await requireLimitTxPair(request, pairs)
    const factoryIndex = pairs.findIndex((p) => p.contract_addr === pairAddr)
    expect(factoryIndex, `seed pair ${pairAddr} on factory list`).toBeGreaterThanOrEqual(0)
    await selectLimitPairByFactoryIndex(page, factoryIndex)

    const claimAll = page.getByTestId('limits-page-claim-all-parked')
    await expect(claimAll).toBeVisible({ timeout: 120_000 })
    await expect(claimAll).toBeEnabled({ timeout: 60_000 })
    await expect(claimAll).toContainText(/\(\s*[2-9]\d*\s*\)/)

    const orderIds = parked.map((r) => r.order_id)

    page.on('dialog', (dialog) => {
      expect(dialog.message()).toContain('LUNC gas')
      expect(dialog.message()).toMatch(/Claim (all \d+|batch \d+ of \d+)/)
      void dialog.accept()
    })

    await claimAll.click()

    const claimResult = page.getByTestId('limits-page-claim-result')
    await assertTxResultAlert(page, 180_000)
    await expect(claimResult.locator('.alert-success')).toBeVisible({ timeout: 30_000 })

    const successAlert = claimResult.locator('.alert-success')
    await expect(successAlert).toContainText(/TX:/i)

    const txHash = await readTxHashFromAlertLink(page, successAlert)
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'claim_expired_limit_orders_batch')).toBe(true)
    }).toPass({ timeout: 180_000 })

    const statusUrl = `${indexerBaseUrl()}/api/v1/pairs/${pairAddr}/limit-placements?status=parked_expired`
    await expect(async () => {
      const res = await request.get(statusUrl, { timeout: 20_000 })
      expect(res.ok()).toBeTruthy()
      const body = (await res.json()) as Array<{ order_id: number; owner: string }>
      const stillParked = body.filter((r) => r.owner === DEV_WALLET && orderIds.includes(r.order_id))
      expect(stillParked).toHaveLength(0)
    }).toPass({ timeout: 120_000 })
  })
})
