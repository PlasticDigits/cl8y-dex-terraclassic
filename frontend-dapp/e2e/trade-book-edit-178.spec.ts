import { test, expect, type Page, type APIRequestContext } from './fixtures/dev-wallet'
import { assertTxResultAlert, skipIfLcdUnreachable } from './helpers/chain'
import { requireLimitTxPair, selectLimitPairByFactoryIndex } from './helpers/limit-e2e'
import { gotoAndCaptureFactoryPairsPage } from './helpers/lcd'

/** EMBER/CORAL pair — hybrid book seed keeps dev-wallet bids here (e2e global setup). */
const SEEDED_PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
const INDEXER = process.env.VITE_INDEXER_URL ?? 'http://127.0.0.1:3001'

async function fetchBidOrder(request: APIRequestContext, orderId: string) {
  const resp = await request.get(`${INDEXER}/api/v1/pairs/${SEEDED_PAIR}/limit-book?side=bid&limit=50`)
  expect(resp.ok()).toBeTruthy()
  const body = (await resp.json()) as {
    orders: Array<{ order_id: number; price: string; remaining: string }>
  }
  const order = body.orders.find((o) => String(o.order_id) === orderId)
  expect(order, `bid #${orderId} on ${SEEDED_PAIR}`).toBeTruthy()
  return order!
}

function fromRawAmount(raw: string, decimals: number): string {
  const whole = BigInt(raw)
  const base = 10n ** BigInt(decimals)
  const intPart = whole / base
  const frac = whole % base
  if (frac === 0n) return intPart.toString()
  const fracStr = frac.toString().padStart(decimals, '0').replace(/0+$/, '')
  return `${intPart}.${fracStr}`
}

async function openSeededTradePair(page: Page, request: import('@playwright/test').APIRequestContext) {
  const pairs = await gotoAndCaptureFactoryPairsPage(page, `/trade/${SEEDED_PAIR}`)
  const hit = await requireLimitTxPair(request, pairs)
  if (hit.pair.contract_addr !== SEEDED_PAIR) {
    await selectLimitPairByFactoryIndex(page, hit.pair.contract_addr)
  }
}

async function waitForOwnedBidEditButton(page: Page) {
  const editBtn = page.locator('[data-testid^="trade-book-edit-bid-"]').first()
  await expect(async () => {
    await expect(editBtn).toBeVisible()
  }).toPass({ timeout: 120_000 })
  return editBtn
}

test.describe.configure({ mode: 'serial' })

test.describe('Trade book Edit prefill (GitLab #178)', () => {
  test('desktop ≥1440px: Edit prefills limit ticket; no tx on Edit; cancel still works', async ({
    page,
    connectWallet,
    request,
  }) => {
    test.setTimeout(360_000)
    await page.setViewportSize({ width: 1440, height: 900 })
    await skipIfLcdUnreachable(request)
    await connectWallet
    await openSeededTradePair(page, request)
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    await expect(page.getByText(/cancel the resting order before placing a replacement/i)).toBeVisible()

    const editBtn = await waitForOwnedBidEditButton(page)
    const editTestId = (await editBtn.getAttribute('data-testid')) ?? ''
    const orderId = editTestId.replace('trade-book-edit-bid-', '')
    const order = await fetchBidOrder(request, orderId)
    const expectedAmount = fromRawAmount(order.remaining, 6)

    await page.getByTestId('trade-order-tab-market').click()
    await expect(page.getByTestId('limit-order-price-input')).toBeHidden()

    const postRequests: string[] = []
    const onRequest = (req: { method: () => string; url: () => string }) => {
      if (req.method() === 'POST') postRequests.push(req.url())
    }
    page.on('request', onRequest)
    await editBtn.click()
    await expect(page.getByTestId('trade-order-tab-limit')).toHaveAttribute('aria-selected', 'true')
    await expect(page.getByTestId('limit-order-price-input')).toHaveValue(order.price)
    await expect(page.getByTestId('limit-order-escrow-amount-input')).toHaveValue(expectedAmount)
    page.off('request', onRequest)
    expect(postRequests, 'Edit alone must not POST (no on-chain amend)').toHaveLength(0)

    page.once('dialog', (dialog) => dialog.accept())
    const cancelBtn = page.getByTestId(`trade-book-cancel-bid-${orderId}`)
    await expect(cancelBtn).toBeVisible()
    await cancelBtn.click()
    await assertTxResultAlert(page)
  })

  test('sub-desktop <1024px: Edit prefills the single visible ticket', async ({ page, connectWallet, request }) => {
    test.setTimeout(360_000)
    await page.setViewportSize({ width: 820, height: 1180 })
    await skipIfLcdUnreachable(request)
    await connectWallet
    await openSeededTradePair(page, request)
    await expect(page.getByTestId('trade-sub-lg-workspace')).toBeVisible({ timeout: 90_000 })

    const editBtn = await waitForOwnedBidEditButton(page)
    const editTestId = (await editBtn.getAttribute('data-testid')) ?? ''
    const orderId = editTestId.replace('trade-book-edit-bid-', '')
    const order = await fetchBidOrder(request, orderId)
    const expectedAmount = fromRawAmount(order.remaining, 6)

    await page.getByTestId('trade-order-tab-market').click()
    await editBtn.click()

    await expect(page.getByTestId('trade-order-tab-limit')).toHaveAttribute('aria-selected', 'true')
    await expect(page.getByTestId('limit-order-price-input')).toHaveValue(order.price)
    await expect(page.getByTestId('limit-order-escrow-amount-input')).toHaveValue(expectedAmount)
  })
})
