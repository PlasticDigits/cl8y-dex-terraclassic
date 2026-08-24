/**
 * Strict community-tax e2e-tx helpers (GitLab #622).
 * Missing LocalTerra tax pins / LCD fail the spec — never `test.skip`.
 */

import { expect, type APIRequestContext, type Locator, type Page } from '@playwright/test'

import { applyBuyTaxNet } from '../../src/utils/communityTaxNetOut'
import { assertLcdReachable, assertSwapCtaNotBlocked } from './chain'
import { requireCommunityTaxTxPins, type CommunityTaxTxPins } from './community-tax-env'
import { E2E_DEV_WALLET } from './fee-discount-quote-e2e'
import { lcdRequestGet } from './lcd-docker-fallback'
import { gotoPoolCardBySymbol } from './pool-nav'
import { clickSwapSubmit, swapActionPanel, swapYouReceiveAmountDisplay } from './swap-ui'

/** Node-safe copies — do not import `formatAmount` / `taxPreviewMaxSpend` (they pull `import.meta.env`). */
export const SELL_TAX_EXTRA_HINT = 'Sell tax extra'
export const BUY_TAX_HINT = 'Buy tax applies'

export function toRawAmount(humanAmount: string, decimals: number): string {
  if (!humanAmount || humanAmount === '0') return '0'
  const [intPart, fracPart = ''] = humanAmount.split('.')
  const paddedFrac = fracPart.padEnd(decimals, '0').slice(0, decimals)
  return (intPart + paddedFrac).replace(/^0+/, '') || '0'
}

function fromRawAmount(rawAmount: string, decimals: number): string {
  if (!rawAmount || rawAmount === '0') return '0'
  const padded = rawAmount.padStart(decimals + 1, '0')
  const intPart = padded.slice(0, padded.length - decimals) || '0'
  const fracPart = padded.slice(padded.length - decimals)
  const trimmedFrac = fracPart.replace(/0+$/, '')
  return trimmedFrac ? `${intPart}.${trimmedFrac}` : intPart
}

export function maxDeclaredForExtraDebitSell(balanceRaw: bigint, sellBps: number): bigint {
  if (balanceRaw <= 0n) return 0n
  const bps = Math.max(0, Math.floor(sellBps))
  if (bps === 0) return balanceRaw
  return (balanceRaw * 10_000n) / BigInt(10_000 + bps)
}

export { E2E_DEV_WALLET }

export const COMMUNITY_TAX_TX_SELL_HUMAN = '0.1'
export const COMMUNITY_TAX_TX_BUY_PAY_HUMAN = '0.1'
export const COMMUNITY_TAX_TX_PROVIDE_HUMAN = '1'
export const COMMUNITY_TAX_TX_LIMIT_HUMAN = '0.001'

const SWAP_HOOK_B64 = Buffer.from(JSON.stringify({ swap: { max_spread: '1' } })).toString('base64')

function decodeSmartDataPayload<T>(raw: { data?: T | string }): T | null {
  const data = raw.data
  if (data == null) return null
  if (typeof data === 'string') {
    try {
      return JSON.parse(Buffer.from(data, 'base64').toString('utf8')) as T
    } catch {
      return null
    }
  }
  return data as T
}

function b64SmartQuery(msg: Record<string, unknown>): string {
  return Buffer.from(JSON.stringify(msg)).toString('base64')
}

export async function lcdSmartQuery<T>(
  request: APIRequestContext,
  contract: string,
  msg: Record<string, unknown>
): Promise<T> {
  const q = b64SmartQuery(msg)
  const res = await lcdRequestGet(request, `/cosmwasm/wasm/v1/contract/${contract}/smart/${q}`, {
    timeout: 20_000,
  })
  expect(res.ok, `LCD smart query ${contract} ${JSON.stringify(msg)} → ${res.status}`).toBe(true)
  const body = (await res.json()) as { data?: T | string }
  const decoded = decodeSmartDataPayload<T>(body)
  expect(decoded, `LCD smart payload empty for ${contract}`).toBeTruthy()
  return decoded as T
}

export async function queryCw20Balance(request: APIRequestContext, token: string, address: string): Promise<bigint> {
  const data = await lcdSmartQuery<{ balance: string }>(request, token, { balance: { address } })
  return BigInt(data.balance)
}

export type LcdTaxPreview = {
  kind: string
  declared: string
  debit: string
  credit: string
  tax: string
}

export async function queryTaxPreview(
  request: APIRequestContext,
  token: string,
  input: { from: string; to: string; amount: string; sendMsg?: string }
): Promise<LcdTaxPreview> {
  return lcdSmartQuery<LcdTaxPreview>(request, token, {
    tax_preview: {
      from: input.from,
      to: input.to,
      amount: input.amount,
      send_msg: input.sendMsg ?? null,
    },
  })
}

export async function queryCommunityTaxConfig(
  request: APIRequestContext,
  token: string
): Promise<{ sell_bps: number; buy_bps: number; treasury: string }> {
  return lcdSmartQuery(request, token, { get_config: {} })
}

export async function queryTokenDecimals(request: APIRequestContext, token: string): Promise<number> {
  const info = await lcdSmartQuery<{ decimals: number }>(request, token, { token_info: {} })
  return info.decimals
}

/** LCD + local seed pins. Never skip — skip-tax deploy must fail this spec (**E622-2**). */
export async function requireCommunityTaxTxContext(
  request: APIRequestContext
): Promise<CommunityTaxTxPins & { sellBps: number; buyBps: number; treasury: string; decimals: number }> {
  await assertLcdReachable(request)
  const pins = requireCommunityTaxTxPins()
  const cfg = await queryCommunityTaxConfig(request, pins.token)
  const decimals = await queryTokenDecimals(request, pins.token)
  expect(cfg.sell_bps, 'QA seed sell_bps').toBeGreaterThan(0)
  expect(cfg.buy_bps, 'QA seed buy_bps').toBeGreaterThan(0)
  return { ...pins, sellBps: cfg.sell_bps, buyBps: cfg.buy_bps, treasury: cfg.treasury, decimals }
}

export async function selectTokenByContract(
  page: Page,
  label: 'Select token you pay' | 'Select token you receive',
  contract: string
): Promise<void> {
  const trigger = page.getByRole('combobox', { name: label })
  await expect(trigger).toBeEnabled({ timeout: 25_000 })
  await page.keyboard.press('Escape')
  await trigger.blur()
  await trigger.click()
  const opt = page.getByTestId(`token-option-${contract}`)
  if (!(await opt.isVisible().catch(() => false))) {
    await trigger.click()
  }
  await expect(
    opt,
    `token option ${contract} missing from ${label} — seed QTAX must be in the factory list`
  ).toBeVisible({ timeout: 60_000 })
  await opt.click()
}

export async function dismissGettingStartedIfPresent(page: Page): Promise<void> {
  const dismiss = page.getByRole('button', { name: 'Dismiss getting started tips' })
  if (await dismiss.isVisible().catch(() => false)) {
    await dismiss.click()
  }
}

export async function clickSwapPayMax(page: Page): Promise<void> {
  const max = page.getByTestId('swap-pay-max')
  await expect(max).toBeVisible({ timeout: 30_000 })
  await expect(max).toBeEnabled({ timeout: 30_000 })
  await max.click()
}

export function swapYouPayInput(page: Page) {
  return page.getByRole('textbox', { name: 'You Pay' })
}

export function parseDisplayedTokenAmount(text: string): number {
  const cleaned = text.replace(/,/g, '').trim()
  const n = Number.parseFloat(cleaned)
  expect(Number.isFinite(n), `could not parse displayed amount "${text}"`).toBe(true)
  return n
}

export function extraDebitSellTax(declared: bigint, sellBps: number): bigint {
  return (declared * BigInt(sellBps)) / 10_000n
}

export function assertMaxIsExtraDebit(params: {
  youPayHuman: string
  walletRaw: bigint
  decimals: number
  sellBps: number
}): void {
  const filledRaw = BigInt(toRawAmount(params.youPayHuman, params.decimals))
  const expected = maxDeclaredForExtraDebitSell(params.walletRaw, params.sellBps)
  expect(filledRaw, 'Max must leave extra-debit room (not 100% of wallet)').toBeLessThan(params.walletRaw)
  expect(filledRaw, 'Max declared must match extra-debit cap').toBe(expected)
  expect(filledRaw + extraDebitSellTax(filledRaw, params.sellBps)).toBeLessThanOrEqual(params.walletRaw)
}

export async function waitForSwapQuoteReady(page: Page): Promise<void> {
  const panel = swapActionPanel(page)
  const youReceive = swapYouReceiveAmountDisplay(page)
  await expect(youReceive).not.toHaveText(/^0(\.0+)?$/, { timeout: 90_000 })
  await expect(async () => {
    const calculating = panel.getByRole('button', { name: /^(Calculating|Searching)/ })
    expect(await calculating.count()).toBe(0)
  }).toPass({ timeout: 120_000 })
}

export async function submitSwapAndReadHash(page: Page): Promise<string> {
  const panel = swapActionPanel(page)
  const swapAction = panel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
  await expect(swapAction).toBeVisible({ timeout: 60_000 })
  assertSwapCtaNotBlocked(await swapAction.textContent())
  await clickSwapSubmit(page, panel)
  const success = panel.locator('.alert-success').first()
  await expect(success, 'expected swap success alert').toBeVisible({ timeout: 120_000 })
  const link = success.locator('a[title]')
  await expect(link).toBeVisible()
  const hash = (await link.getAttribute('title'))?.trim() ?? ''
  expect(hash).toMatch(/^[0-9A-Fa-f]{64}$/)
  return hash
}

export async function querySellSwapPreview(
  request: APIRequestContext,
  token: string,
  pair: string,
  amountRaw: string
): Promise<LcdTaxPreview> {
  return queryTaxPreview(request, token, {
    from: E2E_DEV_WALLET,
    to: pair,
    amount: amountRaw,
    sendMsg: SWAP_HOOK_B64,
  })
}

export function assertSellLcdDeltas(params: {
  userBefore: bigint
  userAfter: bigint
  pairBefore: bigint
  pairAfter: bigint
  sinkBefore: bigint
  sinkAfter: bigint
  preview: LcdTaxPreview
}): void {
  const userDebit = params.userBefore - params.userAfter
  const pairCredit = params.pairAfter - params.pairBefore
  const sinkCredit = params.sinkAfter - params.sinkBefore
  expect(userDebit.toString(), 'sell user debit == TaxPreview.debit (not 1:1)').toBe(params.preview.debit)
  expect(pairCredit.toString(), 'sell pair inbound stays 1:1 (Send amount)').toBe(params.preview.credit)
  expect(pairCredit.toString(), 'pair credit == declared Send').toBe(params.preview.declared)
  expect(userDebit, 'sell extra-debit: user debit > pair credit').toBeGreaterThan(pairCredit)
  expect(sinkCredit.toString(), 'sell tax lands in treasury').toBe(params.preview.tax)
}

export function assertBuyLcdDeltas(params: {
  userBefore: bigint
  userAfter: bigint
  pairBefore: bigint
  pairAfter: bigint
  sinkBefore: bigint
  sinkAfter: bigint
  buyBps: number
}): { pairDebit: bigint; userCredit: bigint; sinkCredit: bigint } {
  const pairDebit = params.pairBefore - params.pairAfter
  const userCredit = params.userAfter - params.userBefore
  const sinkCredit = params.sinkAfter - params.sinkBefore
  expect(pairDebit, 'buy pair debit').toBeGreaterThan(0n)
  expect(userCredit, 'buy user credit (net)').toBeGreaterThan(0n)
  expect(userCredit + sinkCredit, 'user + sink == pair debit').toBe(pairDebit)
  expect(userCredit.toString(), 'You Receive net != raw pair debit').toBe(
    applyBuyTaxNet(pairDebit.toString(), params.buyBps)
  )
  return { pairDebit, userCredit, sinkCredit }
}

export function assertYouReceiveMatchesNetCredit(
  displayed: string,
  userCreditRaw: bigint,
  pairDebitRaw: bigint,
  decimals: number
): void {
  expect(userCreditRaw, 'buy You Receive is net — user credit < pair debit').toBeLessThan(pairDebitRaw)
  const shown = parseDisplayedTokenAmount(displayed.replace(/[A-Za-z]/g, '').trim())
  const netHuman = Number.parseFloat(fromRawAmount(userCreditRaw.toString(), decimals))
  const rawHuman = Number.parseFloat(fromRawAmount(pairDebitRaw.toString(), decimals))
  expect(shown, 'You Receive must match LCD user credit (net), not raw pair debit').toBeCloseTo(netHuman, 3)
  if (rawHuman !== netHuman) {
    expect(shown, 'You Receive must not equal pre-tax pair debit').not.toBeCloseTo(rawHuman, 8)
  }
}

export type LcdPairInfoQuery = {
  liquidity_token?: string
  asset_infos?: Array<{ token?: { contract_addr?: string } }>
}

export async function queryPairInfo(request: APIRequestContext, pair: string): Promise<LcdPairInfoQuery> {
  return lcdSmartQuery<LcdPairInfoQuery>(request, pair, { pair: {} })
}

export async function queryPairLiquidityToken(request: APIRequestContext, pair: string): Promise<string> {
  const info = await queryPairInfo(request, pair)
  const lp = info.liquidity_token?.trim() ?? ''
  expect(lp.startsWith('terra1'), 'pair.liquidity_token').toBe(true)
  return lp
}

/** Ask escrows token0; bid escrows token1. Pick the side that offers the tax token. */
export function limitSideOfferingTax(pairInfo: LcdPairInfoQuery, taxToken: string): 'ask' | 'bid' {
  const a0 = pairInfo.asset_infos?.[0]?.token?.contract_addr?.trim() ?? ''
  const a1 = pairInfo.asset_infos?.[1]?.token?.contract_addr?.trim() ?? ''
  if (a0 === taxToken) return 'ask'
  if (a1 === taxToken) return 'bid'
  throw new Error(`tax token ${taxToken} is not a leg of the pair (${a0}/${a1})`)
}

export async function gotoTaxEmberPoolCard(page: Page, pins: CommunityTaxTxPins): Promise<Locator> {
  return gotoPoolCardBySymbol(page, pins.symbol, {
    goto: true,
    searchQuery: pins.pair,
  })
}

export async function attachSuccessScreenshot(page: Page, name: string): Promise<void> {
  await page.screenshot({ path: `test-results/${name}.png`, fullPage: true })
}
