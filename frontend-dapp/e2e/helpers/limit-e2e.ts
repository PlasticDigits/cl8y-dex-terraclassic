import { test, expect, type Page } from '@playwright/test'
import type { APIRequestContext } from '@playwright/test'

import { isChainOptional } from './chain'
import { firstUnpausedDualCwPair, type LcdPairInfo } from './lcd'
import { skipOrFailIfPairPaused } from './hybrid-e2e'

const DEFAULT_LIMIT_PAIR_MSG =
  'No unpaused dual-CW20 pair on factory; run bash scripts/deploy-dex-local.sh with LocalTerra up (see docs/testing.md § E2E, GitLab #195).'

/** Strict default: fail when every dual-CW20 pair is paused or missing. Optional chain skips instead. */
export async function requireLimitTxPair(
  request: APIRequestContext,
  pairs: LcdPairInfo[],
  detail = DEFAULT_LIMIT_PAIR_MSG
): Promise<{ pair: LcdPairInfo; index: number }> {
  const hit = await firstUnpausedDualCwPair(request, pairs)
  if (hit) return hit
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(hit, detail).toBeTruthy()
  return hit as { pair: LcdPairInfo; index: number }
}

/**
 * Select a factory pair in the Limits page pair combobox by contract address.
 * The dropdown lists pairs by indexer 24h volume, not factory array order.
 */
export async function selectLimitPairByFactoryIndex(page: Page, pairAddress: string): Promise<void> {
  const pairTrigger = page.locator('#limit-pair')
  await expect(pairTrigger).toBeVisible({ timeout: 60_000 })
  await expect(pairTrigger).toBeEnabled({ timeout: 60_000 })
  await pairTrigger.click()
  await pairTrigger.fill(pairAddress)
  const addrSuffix = pairAddress.slice(-6)
  const option = page.getByRole('option').filter({ hasText: addrSuffix })
  await expect(option.first()).toBeVisible({ timeout: 30_000 })
  await option.first().click()
  await expect(pairTrigger).toHaveValue(/\//, { timeout: 30_000 })
  await skipOrFailIfPairPaused(page)
}

/** Select bid or ask on the Limits page ticket. */
export async function selectLimitSide(page: Page, side: 'bid' | 'ask'): Promise<void> {
  await page.getByTestId(`limit-orders-side-${side}`).click()
}

const PLACE_CTA_MSG =
  'Place limit CTA blocked after E2E provisioning; verify scripts/e2e-provision-dev-wallet.sh (GitLab #195).'

/** Place limit submit must not show Insufficient Balance / Connect after global setup. */
export function assertLimitPlaceCtaNotBlocked(label: string | null, detail = PLACE_CTA_MSG): void {
  expect(label, detail).not.toMatch(/Insufficient Balance|Connect Wallet|Connect/i)
}

/** Retail copy is `Ref 1.01259`; older builds used `Current: 1.01259` (#625 leftover #2). */
const LIMIT_PRICE_REF_RE = /(?:Current[^:]*:\s*|Ref\s+)([\d.]+)/

/** UI is `Order #7` — the old `/order #/` regex is case-sensitive and never matches. */
const LAST_PLACED_ORDER_ID_RE = /order #(\d+)/i

export function parseLastPlacedOrderId(text: string): number | null {
  const match = text.match(LAST_PLACED_ORDER_ID_RE)
  if (!match) return null
  const id = Number.parseInt(match[1]!, 10)
  return Number.isFinite(id) && id > 0 ? id : null
}

/**
 * Fill a limit price that passes the place gate for the given side (GitLab #154 / #195).
 * Default UI price is "1", which often blocks bids when reference is below 1.
 */
export async function fillValidLimitPrice(page: Page, side: 'bid' | 'ask' = 'bid'): Promise<void> {
  const priceInput = page.getByTestId('limit-order-price-input')
  await expect(priceInput).toBeVisible({ timeout: 60_000 })
  const context = page.getByTestId('limit-order-price-context')

  let ref = 0
  await expect(async () => {
    const text = (await context.textContent()) ?? ''
    const match = text.match(LIMIT_PRICE_REF_RE)
    if (!match) throw new Error('limit price reference not ready')
    ref = Number.parseFloat(match[1])
    if (!(ref > 0) || !Number.isFinite(ref)) throw new Error('invalid limit price reference')
  }).toPass({ timeout: 60_000 })

  const limit = side === 'bid' ? ref * 0.5 : ref * 2
  await priceInput.fill(String(limit))
}

/** More conservative price when book-walk cap rejects a tight bid/ask (post-hybrid E2E). */
export async function fillConservativeLimitPrice(page: Page, side: 'bid' | 'ask' = 'bid'): Promise<void> {
  const priceInput = page.getByTestId('limit-order-price-input')
  await expect(priceInput).toBeVisible({ timeout: 60_000 })
  const context = page.getByTestId('limit-order-price-context')

  let ref = 0
  await expect(async () => {
    const text = (await context.textContent()) ?? ''
    const match = text.match(LIMIT_PRICE_REF_RE)
    if (!match) throw new Error('limit price reference not ready')
    ref = Number.parseFloat(match[1])
    if (!(ref > 0) || !Number.isFinite(ref)) throw new Error('invalid limit price reference')
  }).toPass({ timeout: 60_000 })

  const limit = side === 'bid' ? ref * 0.25 : ref * 4
  await priceInput.fill(String(limit))
}

/** Place-limit card scoped to Limits page / trade ticket. */
export function placeLimitCard(page: Page) {
  return page.locator('.card-glass').filter({ has: page.getByRole('button', { name: /^Place limit$/i }) })
}

export type PlacementGasPresetTier = 'Low' | 'Medium' | 'High' | 'Custom'

/** Expand Advanced and pick a placement gas preset (GitLab #204). */
export async function selectPlacementGasPreset(
  page: Page,
  card: ReturnType<typeof placeLimitCard>,
  tier: PlacementGasPresetTier,
  customSteps?: number
): Promise<void> {
  const details = card.locator('details').filter({ hasText: 'Placement gas (book walk)' })
  await expect(details).toBeVisible({ timeout: 30_000 })
  const presetGroup = card.getByRole('group', { name: 'Placement gas preset' })
  if (!(await presetGroup.isVisible().catch(() => false))) {
    await details.locator('summary').click()
  }
  await expect(presetGroup).toBeVisible({ timeout: 15_000 })
  await presetGroup.getByRole('button', { name: tier, exact: true }).click()
  if (tier === 'Custom') {
    const input = card.locator('input[type="number"]').first()
    await expect(input).toBeVisible({ timeout: 10_000 })
    await input.fill(String(customSteps ?? 64))
  }
}

/**
 * Click Place limit and wait for a success alert with TX hash in the place card.
 * Retries on LocalTerra account sequence races (bot swarm / concurrent E2E).
 */
export async function submitPlaceLimitAndExpectTx(page: Page): Promise<void> {
  const card = placeLimitCard(page)
  const placeBtn = card.getByRole('button', { name: /^Place limit$/i })
  const successAlert = card.locator('.alert-success')

  let attempt = 0
  await expect(async () => {
    attempt += 1
    if (await successAlert.isVisible().catch(() => false)) {
      await expect(successAlert).toContainText(/TX:/i)
      return
    }
    await expect(placeBtn).toBeEnabled({ timeout: 10_000 })
    await placeBtn.click()
    await expect(placeBtn).not.toHaveText(/Placing/i, { timeout: 90_000 })

    try {
      await expect(successAlert).toContainText(/TX:/i, { timeout: 20_000 })
      return
    } catch {
      const err = card.locator('.alert-error')
      const msg = (await err.textContent().catch(() => '')) ?? ''
      if (/account sequence mismatch/i.test(msg)) {
        await page.waitForTimeout(Math.min(attempt * 1_500, 8_000))
        throw new Error('retry after account sequence mismatch')
      }
      if (/book-walk cap|placed no rungs/i.test(msg)) {
        await fillConservativeLimitPrice(page, 'ask')
        await page.waitForTimeout(1_000)
        throw new Error('retry after book-walk cap')
      }
      if (msg.trim()) throw new Error(`place limit failed: ${msg.trim()}`)
      throw new Error('no tx result after place limit click')
    }
  }).toPass({ timeout: 180_000 })
}

export function cancelLimitCard(page: Page) {
  return page.locator('.card-glass').filter({ hasText: 'Cancel limit' })
}

/** My open limits panel on `/limits` (primary cancel path — GitLab #419). */
export function myOpenLimitsPanel(page: Page) {
  return page.getByTestId('limits-my-open-limits')
}

/**
 * Cancel a resting limit via the placements panel row CTA (not Advanced order-id form).
 * Accepts the confirm dialog and waits for the cancel tx success alert.
 */
async function waitForCancelSuccessAlert(page: Page): Promise<void> {
  const pageSuccess = page
    .locator('.alert-success')
    .filter({ hasText: /Cancel (transaction )?submitted/i })
    .first()
  await expect(pageSuccess).toBeVisible({ timeout: 90_000 })
}

export async function submitPanelCancelPlacementAndExpectTx(page: Page, orderId: number): Promise<void> {
  const panel = myOpenLimitsPanel(page)
  const cancelBtn = panel.getByTestId(`limits-page-cancel-placement-${orderId}`)

  await expect(cancelBtn).toBeVisible({ timeout: 120_000 })
  await expect(cancelBtn).toBeEnabled({ timeout: 60_000 })

  page.once('dialog', (dialog) => {
    expect(dialog.message()).toContain(`Cancel order #${orderId}`)
    void dialog.accept()
  })
  await cancelBtn.scrollIntoViewIfNeeded()
  await cancelBtn.click()
  await waitForCancelSuccessAlert(page)
}

/** Advanced order-id form — does not wait on indexer My placements or a clogged book. */
export async function submitCancelLimitForOrderAndExpectTx(page: Page, orderId: number): Promise<void> {
  await submitAdvancedCancelByOrderIdAndExpectTx(page, orderId)
}

/**
 * Cancel by LCD `order_id` via Advanced **Cancel by order ID**.
 * Prefer this on leftover tax/EMBER runs: `last-placed-order-id` is indexer maxId
 * (includes cancelled leftovers) so the My placements Cancel row can be missing.
 */
export async function submitAdvancedCancelByOrderIdAndExpectTx(page: Page, orderId: number): Promise<void> {
  const details = page.locator('details.card-glass').filter({ hasText: 'Cancel by order ID' })
  await expect(details).toBeVisible({ timeout: 30_000 })
  await details.evaluate((el) => {
    ;(el as HTMLDetailsElement).open = true
  })
  const input = details.getByRole('textbox', { name: 'Order ID' })
  await expect(input).toBeVisible({ timeout: 15_000 })
  await input.fill(String(orderId))
  await expect(input).toHaveValue(String(orderId))
  const cancelBtn = details.getByRole('button', { name: /^Cancel limit$/i })
  await expect(cancelBtn).toBeEnabled({ timeout: 15_000 })
  await cancelBtn.scrollIntoViewIfNeeded()
  await cancelBtn.click()
  await waitForCancelSuccessAlert(page)
}

/** Click Place ladder and wait for TX success (retries LocalTerra account sequence races). */
export async function submitLadderPlaceAndExpectTx(page: Page): Promise<void> {
  const ladderPanel = page.getByTestId('limit-order-ladder-panel')
  const ladderBtn = ladderPanel.getByTestId('ladder-place-submit')
  const successAlert = ladderPanel.locator('.alert-success')
  const errorAlert = ladderPanel.locator('.alert-error')

  let attempt = 0
  await expect(async () => {
    attempt += 1
    if (await successAlert.isVisible().catch(() => false)) {
      await expect(successAlert).toContainText(/TX:/i)
      return
    }
    // Allowance + ladder = two txs; wait out a prior in-flight attempt before re-clicking.
    await expect(ladderBtn).toBeEnabled({ timeout: 180_000 })
    await expect(ladderBtn).not.toHaveText(/Placing ladder/i, { timeout: 5_000 })
    await ladderBtn.click()
    await expect(ladderBtn).not.toHaveText(/Placing ladder/i, { timeout: 180_000 })

    try {
      await expect(successAlert).toContainText(/TX:/i, { timeout: 45_000 })
      return
    } catch {
      const msg = (await errorAlert.textContent().catch(() => '')) ?? ''
      if (/account sequence mismatch/i.test(msg)) {
        await page.waitForTimeout(Math.min(attempt * 2_000, 12_000))
        throw new Error('retry after account sequence mismatch')
      }
      if (msg.trim()) throw new Error(`ladder place failed: ${msg.trim()}`)
      throw new Error('no tx result after ladder place click')
    }
  }).toPass({ timeout: 300_000 })
}

/** Click Cancel limit and wait for TX success in the cancel card (retries sequence mismatch). */
export async function submitCancelLimitAndExpectTx(page: Page): Promise<void> {
  const card = cancelLimitCard(page)
  const cancelBtn = card.getByRole('button', { name: /^Cancel limit$/i })
  const successAlert = card.locator('.alert-success')

  let attempt = 0
  await expect(async () => {
    attempt += 1
    if (await successAlert.isVisible().catch(() => false)) {
      await expect(successAlert).toContainText(/TX:/i)
      return
    }
    await expect(cancelBtn).toBeEnabled({ timeout: 10_000 })
    await cancelBtn.click()
    await expect(cancelBtn).not.toHaveText(/Cancelling/i, { timeout: 90_000 })

    try {
      await expect(successAlert).toContainText(/TX:/i, { timeout: 20_000 })
      return
    } catch {
      const err = card.locator('.alert-error')
      const msg = (await err.textContent().catch(() => '')) ?? ''
      if (/account sequence mismatch/i.test(msg)) {
        await page.waitForTimeout(Math.min(attempt * 1_500, 8_000))
        throw new Error('retry after account sequence mismatch')
      }
      if (msg.trim()) throw new Error(`cancel limit failed: ${msg.trim()}`)
      throw new Error('no tx result after cancel limit click')
    }
  }).toPass({ timeout: 180_000 })
}
