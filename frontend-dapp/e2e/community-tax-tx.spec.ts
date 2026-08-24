/**
 * Playwright e2e-tx matrix for the LocalTerra community-tax / EMBER pair (GitLab #622).
 *
 * Strict: missing seed pins or LCD **fails** (no `test.skip`). e2e-tx only (1 worker).
 * Do not turn hybrid off (#596). Wrap stays cLUNC/cUSTC. Gem specs stay on EMBER/CORAL.
 */

import { expect, test } from './fixtures/dev-wallet'
import { assertLiquidityCtaNotBlocked } from './helpers/chain'
import {
  assertBuyLcdDeltas,
  assertCancelLimitRefundIsBuyNotSell,
  assertMaxIsExtraDebit,
  assertPlaceLimitNoSellExtraDebit,
  assertSellLcdDeltas,
  assertYouReceiveMatchesNetCredit,
  attachSuccessScreenshot,
  BUY_TAX_HINT,
  clickSwapPayMax,
  COMMUNITY_TAX_TX_BUY_PAY_HUMAN,
  COMMUNITY_TAX_TX_LIMIT_HUMAN,
  COMMUNITY_TAX_TX_PROVIDE_HUMAN,
  COMMUNITY_TAX_TX_SELL_HUMAN,
  dismissGettingStartedIfPresent,
  E2E_DEV_WALLET,
  extraDebitSellTax,
  gotoTaxEmberPoolCard,
  queryCw20Balance,
  limitSideOfferingTax,
  queryPairInfo,
  querySellSwapPreview,
  requireCommunityTaxTxContext,
  selectTokenByContract,
  SELL_TAX_EXTRA_HINT,
  submitSwapAndReadHash,
  swapYouPayInput,
  waitForSwapQuoteReady,
} from './helpers/community-tax-e2e'
import {
  assertLimitPlaceCtaNotBlocked,
  fillValidLimitPrice,
  placeLimitCard,
  selectLimitPairByFactoryIndex,
  selectLimitSide,
  submitCancelLimitForOrderAndExpectTx,
  submitPlaceLimitAndExpectTx,
} from './helpers/limit-e2e'
import { fetchTxJson, readTxHashFromAlertLink, txJsonHasWasmAction, txJsonWasmAttrForAction } from './helpers/lcd'
import { openPoolCardAdvanced, poolProvideExpandButton, poolProvideSubmitButton } from './helpers/pool-ui'
import { openSwapSettingsAndSetSlippage, swapYouReceiveAmountDisplay } from './helpers/swap-ui'
import { headerConnectedWalletButton } from './helpers/wallet-ui'
import { toRawAmount } from './helpers/community-tax-e2e'

test.describe.configure({ mode: 'serial' })

test.describe('Community-tax pair e2e-tx (GitLab #622)', () => {
  test('P0 sell QTAX → EMBER: Max extra-debit + TaxPreview debit', async ({ page, connectWallet, request }) => {
    test.setTimeout(240_000)
    const ctx = await requireCommunityTaxTxContext(request)
    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible({ timeout: 15_000 })
    await dismissGettingStartedIfPresent(page)

    await selectTokenByContract(page, 'Select token you pay', ctx.token)
    await selectTokenByContract(page, 'Select token you receive', ctx.ember)
    await openSwapSettingsAndSetSlippage(page, 15)

    const walletRaw = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    expect(walletRaw, 'tax balance after Transfer funding (#620)').toBeGreaterThan(0n)

    await clickSwapPayMax(page)
    const maxHuman = (await swapYouPayInput(page).inputValue()).trim()
    assertMaxIsExtraDebit({
      youPayHuman: maxHuman,
      walletRaw,
      decimals: ctx.decimals,
      sellBps: ctx.sellBps,
    })
    await expect(page.getByTestId('swap-sell-tax-extra')).toContainText(SELL_TAX_EXTRA_HINT)

    // Do not submit Max — seed LP is ~10M raw / side. Use a small declared amount.
    await swapYouPayInput(page).fill(COMMUNITY_TAX_TX_SELL_HUMAN)
    await waitForSwapQuoteReady(page)
    await expect(page.getByTestId('swap-sell-tax-extra')).toContainText(SELL_TAX_EXTRA_HINT)

    const declaredRaw = toRawAmount(COMMUNITY_TAX_TX_SELL_HUMAN, ctx.decimals)
    const preview = await querySellSwapPreview(request, ctx.token, ctx.pair, declaredRaw)
    expect(preview.kind.toLowerCase()).toBe('sell')
    expect(preview.declared).toBe(declaredRaw)
    expect(BigInt(preview.debit)).toBe(BigInt(declaredRaw) + extraDebitSellTax(BigInt(declaredRaw), ctx.sellBps))
    expect(preview.credit).toBe(declaredRaw)

    const userBefore = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairBefore = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkBefore = await queryCw20Balance(request, ctx.token, ctx.treasury)

    const txHash = await submitSwapAndReadHash(page)
    await attachSuccessScreenshot(page, 'community-tax-sell')

    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'swap')).toBe(true)
    }).toPass({ timeout: 180_000 })

    const userAfter = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairAfter = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkAfter = await queryCw20Balance(request, ctx.token, ctx.treasury)
    assertSellLcdDeltas({
      userBefore,
      userAfter,
      pairBefore,
      pairAfter,
      sinkBefore,
      sinkAfter,
      preview,
    })
  })

  test('P0 buy EMBER → QTAX: You Receive is net', async ({ page, connectWallet, request }) => {
    test.setTimeout(240_000)
    const ctx = await requireCommunityTaxTxContext(request)
    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible({ timeout: 15_000 })
    await dismissGettingStartedIfPresent(page)

    await selectTokenByContract(page, 'Select token you pay', ctx.ember)
    await selectTokenByContract(page, 'Select token you receive', ctx.token)
    await openSwapSettingsAndSetSlippage(page, 15)
    await swapYouPayInput(page).fill(COMMUNITY_TAX_TX_BUY_PAY_HUMAN)
    await waitForSwapQuoteReady(page)
    await expect(page.getByTestId('swap-sell-tax-extra')).toContainText(BUY_TAX_HINT)

    const youReceiveText = ((await swapYouReceiveAmountDisplay(page).textContent()) ?? '').trim()
    expect(youReceiveText).not.toMatch(/^0(\.0+)?$/)

    const userBefore = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairBefore = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkBefore = await queryCw20Balance(request, ctx.token, ctx.treasury)

    const txHash = await submitSwapAndReadHash(page)
    await attachSuccessScreenshot(page, 'community-tax-buy')

    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'swap')).toBe(true)
    }).toPass({ timeout: 180_000 })

    const userAfter = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairAfter = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkAfter = await queryCw20Balance(request, ctx.token, ctx.treasury)
    const { pairDebit, userCredit } = assertBuyLcdDeltas({
      userBefore,
      userAfter,
      pairBefore,
      pairAfter,
      sinkBefore,
      sinkAfter,
      buyBps: ctx.buyBps,
    })
    assertYouReceiveMatchesNetCredit(youReceiveText, userCredit, pairDebit, ctx.decimals)
  })

  test('P0 provide tax/EMBER: TransferFrom 1:1', async ({ page, connectWallet, request }) => {
    test.setTimeout(300_000)
    const ctx = await requireCommunityTaxTxContext(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)

    const pairCard = await gotoTaxEmberPoolCard(page, ctx)
    await openPoolCardAdvanced(pairCard)
    await expect(poolProvideExpandButton(pairCard)).toBeVisible({ timeout: 90_000 })
    await poolProvideExpandButton(pairCard).click()

    const inputs = pairCard.locator('input[placeholder="0.00"]')
    await inputs.nth(0).fill(COMMUNITY_TAX_TX_PROVIDE_HUMAN)
    await inputs.nth(1).fill(COMMUNITY_TAX_TX_PROVIDE_HUMAN)

    const declaredRaw = BigInt(toRawAmount(COMMUNITY_TAX_TX_PROVIDE_HUMAN, ctx.decimals))
    const pairTaxBefore = await queryCw20Balance(request, ctx.token, ctx.pair)
    const pairEmberBefore = await queryCw20Balance(request, ctx.ember, ctx.pair)

    const submitBtn = poolProvideSubmitButton(pairCard)
    await expect(submitBtn).toBeEnabled({ timeout: 15_000 })
    assertLiquidityCtaNotBlocked(await submitBtn.textContent(), 'tax/EMBER provide CTA blocked')
    await submitBtn.scrollIntoViewIfNeeded()
    await submitBtn.click()
    const provideAlert = pairCard.locator('.alert-success').first()
    await expect(provideAlert).toBeVisible({ timeout: 120_000 })

    const pairTaxAfter = await queryCw20Balance(request, ctx.token, ctx.pair)
    const pairEmberAfter = await queryCw20Balance(request, ctx.ember, ctx.pair)
    expect(pairTaxAfter - pairTaxBefore, 'provide tax TransferFrom 1:1').toBe(declaredRaw)
    expect(pairEmberAfter - pairEmberBefore, 'provide EMBER TransferFrom 1:1').toBe(declaredRaw)
  })

  test('P0 limit place + cancel on tax/EMBER: escrow 1:1', async ({ page, connectWallet, request }) => {
    test.setTimeout(420_000)
    const ctx = await requireCommunityTaxTxContext(request)
    await connectWallet

    await page.goto('/limits')
    await expect(page.locator('#limit-pair')).toBeVisible({ timeout: 60_000 })
    await selectLimitPairByFactoryIndex(page, ctx.pair)
    const pairInfo = await queryPairInfo(request, ctx.pair)
    const offerSide = limitSideOfferingTax(pairInfo, ctx.token)
    await selectLimitSide(page, offerSide)

    const placeCard = placeLimitCard(page)
    await fillValidLimitPrice(page, offerSide)
    await placeCard.getByPlaceholder('0.0').fill(COMMUNITY_TAX_TX_LIMIT_HUMAN)
    const placeBtn = placeCard.getByRole('button', { name: /^Place limit$/i })
    await expect(placeBtn).toBeVisible({ timeout: 60_000 })
    assertLimitPlaceCtaNotBlocked(await placeBtn.textContent())

    const declaredRaw = BigInt(toRawAmount(COMMUNITY_TAX_TX_LIMIT_HUMAN, ctx.decimals))
    const userBefore = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairBefore = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkBefore = await queryCw20Balance(request, ctx.token, ctx.treasury)

    await submitPlaceLimitAndExpectTx(page)
    const successAlert = placeCard.locator('.alert-success')
    const txHash = await readTxHashFromAlertLink(page, successAlert)
    let placeJson: unknown
    await expect(async () => {
      const json = await fetchTxJson(request, txHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'place_limit_order')).toBe(true)
      placeJson = json
    }).toPass({ timeout: 180_000 })

    const userAfterPlace = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairAfterPlace = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkAfterPlace = await queryCw20Balance(request, ctx.token, ctx.treasury)
    const makerFee = BigInt(txJsonWasmAttrForAction(placeJson, 'place_limit_order', 'maker_fee_amount') ?? '0')
    assertPlaceLimitNoSellExtraDebit({
      userDebit: userBefore - userAfterPlace,
      pairCredit: pairAfterPlace - pairBefore,
      declaredRaw,
      makerFee,
    })
    expect(sinkAfterPlace, 'place Send is honest — no sell tax to treasury').toBe(sinkBefore)

    const orderId = Number(txJsonWasmAttrForAction(placeJson, 'place_limit_order', 'order_id') ?? '0')
    expect(orderId, 'place_limit_order order_id').toBeGreaterThan(0)

    await submitCancelLimitForOrderAndExpectTx(page, orderId)
    const cancelSuccess = page
      .locator('.alert-success')
      .filter({ hasText: /Cancel (transaction )?submitted/i })
      .first()
    const cancelHash = await readTxHashFromAlertLink(page, cancelSuccess)
    await expect(async () => {
      const json = await fetchTxJson(request, cancelHash)
      if (!json) throw new Error('LCD tx not indexed yet')
      expect(txJsonHasWasmAction(json, 'cancel_limit_order')).toBe(true)
    }).toPass({ timeout: 180_000 })

    const userAfterCancel = await queryCw20Balance(request, ctx.token, E2E_DEV_WALLET)
    const pairAfterCancel = await queryCw20Balance(request, ctx.token, ctx.pair)
    const sinkAfterCancel = await queryCw20Balance(request, ctx.token, ctx.treasury)
    assertCancelLimitRefundIsBuyNotSell({
      userAfterPlace,
      userAfterCancel,
      pairAfterPlace,
      pairAfterCancel,
      sinkAfterPlace,
      sinkAfterCancel,
      remainingEscrow: declaredRaw - makerFee,
      buyBps: ctx.buyBps,
    })
  })

  test('P1 Trade Market GET /route/solve You Receive is net (R615)', async ({ page, connectWallet, request }) => {
    test.setTimeout(180_000)
    const ctx = await requireCommunityTaxTxContext(request)
    await connectWallet

    const getSolves: string[] = []
    page.on('request', (req) => {
      if (req.method() === 'GET' && req.url().includes('/api/v1/route/solve')) getSolves.push(req.url())
    })

    await page.goto(`/trade/${ctx.pair}`)
    await page.waitForLoadState('networkidle')
    await page.getByTestId('trade-order-tab-market').click()
    await page.getByTestId('limit-order-escrow-amount-input').fill(COMMUNITY_TAX_TX_BUY_PAY_HUMAN)
    await expect(page.getByTestId('trade-market-quote')).toBeVisible({ timeout: 90_000 })
    await expect.poll(() => getSolves.length, { timeout: 60_000 }).toBeGreaterThan(0)
    await expect(page.getByTestId('trade-market-expected-receive')).not.toHaveText(/Quoting/i)
    const receiveText = ((await page.getByTestId('trade-market-expected-receive').textContent()) ?? '').trim()
    expect(receiveText.length, 'Trade Market expected receive').toBeGreaterThan(0)
    await expect(page.locator('body')).not.toContainText('Route skips buy/sell tax')
  })
})
