import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import {
  assertLimitPlaceCtaNotBlocked,
  fillValidLimitPrice,
  placeLimitCard,
  requireLimitTxPair,
  selectLimitPairByFactoryIndex,
  selectPlacementGasPreset,
  submitPlaceLimitAndExpectTx,
} from './helpers/limit-e2e'
import {
  fetchTxJson,
  gotoAndCaptureFactoryPairsPage,
  readTxHashFromAlertLink,
  txJsonPlaceLimitMaxAdjustSteps,
} from './helpers/lcd'

/** GitLab #204 — each UI preset must wire the mapped `max_adjust_steps` in the CW20 batch hook. */
const PRESET_CASES: Array<{ tier: 'Low' | 'Medium' | 'High' | 'Custom'; expectedSteps: number; customSteps?: number }> =
  [
    { tier: 'Low', expectedSteps: 16 },
    { tier: 'Medium', expectedSteps: 32 },
    { tier: 'High', expectedSteps: 128 },
    { tier: 'Custom', expectedSteps: 64, customSteps: 64 },
  ]

test.describe.configure({ mode: 'serial' })

test.describe('Limit placement gas presets → CW20 hook max_adjust_steps (GitLab #204)', () => {
  for (const { tier, expectedSteps, customSteps } of PRESET_CASES) {
    test(`/limits places bid with ${tier} preset → hook max_adjust_steps=${expectedSteps}`, async ({
      page,
      connectWallet,
      request,
    }) => {
      test.setTimeout(300_000)
      await skipIfLcdUnreachable(request)
      await connectWallet

      const pairs = await gotoAndCaptureFactoryPairsPage(page, '/limits')
      const { pair } = await requireLimitTxPair(request, pairs)
      await selectLimitPairByFactoryIndex(page, pair.contract_addr)

      const card = placeLimitCard(page)
      await fillValidLimitPrice(page, 'bid')
      await selectPlacementGasPreset(page, card, tier, customSteps)
      await card.getByPlaceholder('0.0').fill('1')

      const placeBtn = card.getByRole('button', { name: /^Place limit$/i })
      await expect(placeBtn).toBeEnabled({ timeout: 60_000 })
      assertLimitPlaceCtaNotBlocked(await placeBtn.textContent())
      await submitPlaceLimitAndExpectTx(page)

      const successAlert = card.locator('.alert-success')
      const txHash = await readTxHashFromAlertLink(page, successAlert)
      await expect(async () => {
        const json = await fetchTxJson(request, txHash)
        if (!json) throw new Error('LCD tx not indexed yet')
        const steps = txJsonPlaceLimitMaxAdjustSteps(json)
        expect(steps, `CW20 hook max_adjust_steps for ${tier}`).toBe(expectedSteps)
      }).toPass({ timeout: 180_000 })
    })
  }
})
