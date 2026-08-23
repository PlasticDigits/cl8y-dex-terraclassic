/**
 * Shared pay-with-any-token invoice module (GitLab #595).
 *
 * Exact-out settlement: acquire ≥ invoice of a canonical CW20, then `Send` **exactly**
 * the invoice amount to the feature payee with a hook. Excess invoice token and leftover
 * pay-token dust stay with the user. v1 settlement is one wallet multi-msg
 * (`executeTerraContractMulti`) — swap `to` = user, then payee `Send`. An on-chain
 * `invoice-payer` adapter is a follow-up, not required for #593 consumers.
 *
 * Do not fork Swap quote/execute into Create Token / manager settings / #597.
 */

import { getRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import {
  reverseSimulateMultiHopSwap,
  serializeTerraSwap,
  simulateMultiHopSwap,
  type SwapOperation,
} from '@/services/terraclassic/router'
import { netAfterWrapMapperFee } from '@/services/terraclassic/wrapMapper'
import { assetInfoLabel, getWrappedEquivalent, isNativeDenom, tokenAssetInfo } from '@/types'
import {
  isNativeWrapEnabled,
  PAY_INVOICE_HOOK_KEYS,
  ROUTER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
} from '@/utils/constants'
import { shouldRejectGemBridgeQuote } from '@/utils/pairCatalogRank'
import {
  PAY_INVOICE_INSUFFICIENT,
  PAY_INVOICE_INVALID_INVOICE,
  PAY_INVOICE_INVALID_PAYEE,
  PAY_INVOICE_NO_ROUTE,
  PAY_INVOICE_WRAP_UNAVAILABLE,
} from '@/utils/payInvoiceCopy'
import { applySlippagePercentCeiling, isPositiveRawAmount, slippagePercentToBps } from '@/utils/rawAmountMath'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { tokenPathFromSwapOperations } from '@/utils/swapRouteDisplay'
import type { QuoteTraderOptions } from '@/services/terraclassic/pair'

export {
  PAY_INVOICE_HOOK_KEYS,
  PAY_INVOICE_INSUFFICIENT,
  PAY_INVOICE_INVALID_INVOICE,
  PAY_INVOICE_INVALID_PAYEE,
  PAY_INVOICE_NO_ROUTE,
  PAY_INVOICE_WRAP_UNAVAILABLE,
}

/** Canonical invoice a feature passes in. Payee is never taken from the URL. */
export type Invoice = {
  invoiceToken: string
  invoiceAmount: string
  payee: string
  /** Base64 CosmWasm hook (EnableFeature / CreateToken / settings batch). Empty = plain Send. */
  hookMsg: string
}

export type PayInvoiceQuoteKind = 'direct' | 'routed' | 'wrap_routed'

export type PayInvoiceQuoteOk = {
  status: 'ok'
  kind: PayInvoiceQuoteKind
  payToken: string
  /** CW20 offered to the router (wrapped equivalent when pay token is native). */
  offerCw20: string
  /** User debit of `payToken` (native gross or CW20), including slippage buffer. */
  payRaw: string
  /** CW20 amount in the router `Send` (post-wrap when native). */
  cw20SendAmount: string
  /** Same as `payRaw` — reverse-sim offer × (1 + slippage). */
  maxIn: string
  /** Always the invoice amount — never invoice × (1 − slippage). */
  minInvoiceOut: string
  operations: SwapOperation[]
  wrap?: { denom: string; grossNative: string }
  routeLabel: string
  hopCount: number
}

export type PayInvoiceQuote = PayInvoiceQuoteOk | { status: 'unavailable'; disableReason: string }

export type TaxPreviewExtraDebit = (intendedCredit: string) => string | Promise<string>

export type QuotePayInvoiceInput = {
  invoice: Invoice
  payToken: string
  slippagePercent: number
  trader?: string
  payTokenBalance?: string
  wrapFeeBps?: number | null
  /** Tax-token extra debit so the router/payee credit matches (#592). Fail closed if insufficient. */
  taxPreviewExtraDebit?: TaxPreviewExtraDebit
  signal?: AbortSignal
}

export type PayInvoiceQuoteDeps = {
  getRouteSolve: typeof getRouteSolve
  reverseSimulate: typeof reverseSimulateMultiHopSwap
  forwardSimulate: typeof simulateMultiHopSwap
}

const defaultQuoteDeps: PayInvoiceQuoteDeps = {
  getRouteSolve,
  reverseSimulate: reverseSimulateMultiHopSwap,
  forwardSimulate: simulateMultiHopSwap,
}

const FORWARD_BUMP_GUARD = 8

export function sameTokenId(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

/** Payee comes only from feature config (`Invoice.payee`). Query strings are ignored. */
export function resolveInvoicePayee(invoice: Invoice, _search?: string): string {
  void _search
  return invoice.payee.trim()
}

export function validateInvoice(invoice: Invoice): string | null {
  if (!isPositiveRawAmount(invoice.invoiceAmount)) return PAY_INVOICE_INVALID_INVOICE
  if (!isValidTerraBech32Address(invoice.invoiceToken)) return PAY_INVOICE_INVALID_INVOICE
  if (!isValidTerraBech32Address(invoice.payee)) return PAY_INVOICE_INVALID_PAYEE
  if (invoice.hookMsg) {
    try {
      atob(invoice.hookMsg)
    } catch {
      return PAY_INVOICE_INVALID_INVOICE
    }
  }
  return null
}

export function wrapGrossForNetCw20(netCw20: bigint, feeBps: number): bigint {
  const keep = 10000n - BigInt(Math.min(10_000, Math.max(0, Math.trunc(feeBps))))
  if (keep <= 0n) return netCw20
  return (netCw20 * 10000n + keep - 1n) / keep
}

function slippageToMaxSpread(percent: number): string {
  const bps = slippagePercentToBps(percent)
  if (bps <= 0) return '0'
  if (bps >= 10_000) return '1'
  const whole = Math.floor(bps / 10_000)
  const frac = (bps % 10_000).toString().padStart(4, '0').replace(/0+$/, '')
  return frac.length === 0 ? String(whole) : `${whole}.${frac}`
}

function routeLabelFromOps(operations: SwapOperation[], display: (id: string) => string): string {
  return tokenPathFromSwapOperations(operations).map(display).join(' → ')
}

function quoteTrader(trader?: string): QuoteTraderOptions | undefined {
  return trader?.trim() ? { trader: trader.trim() } : undefined
}

async function applyTaxDebit(intended: string, extra?: TaxPreviewExtraDebit): Promise<string> {
  if (!extra) return intended
  const debit = await extra(intended)
  if (!isPositiveRawAmount(debit) || BigInt(debit) < BigInt(intended)) {
    throw new Error(PAY_INVOICE_INSUFFICIENT)
  }
  return debit
}

function balanceShort(balance: string | undefined, need: string): boolean {
  if (balance === undefined) return false
  try {
    return BigInt(balance) < BigInt(need)
  } catch {
    return true
  }
}

async function bumpOfferUntilInvoice(input: {
  offer: string
  operations: SwapOperation[]
  invoiceAmount: string
  trader?: string
  forwardSimulate: PayInvoiceQuoteDeps['forwardSimulate']
}): Promise<string | null> {
  let offer = input.offer
  const trader = quoteTrader(input.trader)
  for (let i = 0; i < FORWARD_BUMP_GUARD; i++) {
    const sim = await input.forwardSimulate(offer, input.operations, trader)
    if (BigInt(sim.amount) >= BigInt(input.invoiceAmount)) return offer
    offer = ((BigInt(offer) * 10001n) / 10000n + 1n).toString()
  }
  return null
}

export async function quotePayInvoice(
  input: QuotePayInvoiceInput,
  deps: PayInvoiceQuoteDeps = defaultQuoteDeps
): Promise<PayInvoiceQuote> {
  const invalid = validateInvoice(input.invoice)
  if (invalid) return { status: 'unavailable', disableReason: invalid }

  const invoice = input.invoice
  const payToken = input.payToken.trim()
  if (!payToken) return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }

  if (sameTokenId(payToken, invoice.invoiceToken)) {
    let payRaw = invoice.invoiceAmount
    try {
      payRaw = await applyTaxDebit(payRaw, input.taxPreviewExtraDebit)
    } catch {
      return { status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT }
    }
    if (balanceShort(input.payTokenBalance, payRaw)) {
      return { status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT }
    }
    return {
      status: 'ok',
      kind: 'direct',
      payToken,
      offerCw20: invoice.invoiceToken,
      payRaw,
      cw20SendAmount: invoice.invoiceAmount,
      maxIn: payRaw,
      minInvoiceOut: invoice.invoiceAmount,
      operations: [],
      routeLabel: '',
      hopCount: 0,
    }
  }

  let offerCw20 = payToken
  let wrapDenom: string | null = null
  if (isNativeDenom(payToken)) {
    if (!isNativeWrapEnabled() || input.wrapFeeBps == null) {
      return { status: 'unavailable', disableReason: PAY_INVOICE_WRAP_UNAVAILABLE }
    }
    const wrapped = getWrappedEquivalent(payToken)
    if (!wrapped) return { status: 'unavailable', disableReason: PAY_INVOICE_WRAP_UNAVAILABLE }
    offerCw20 = wrapped
    wrapDenom = payToken
  }

  let ops: SwapOperation[]
  try {
    const idx = await deps.getRouteSolve(offerCw20, invoice.invoiceToken, invoice.invoiceAmount, {
      trader: input.trader,
      signal: input.signal,
    })
    const hopTokens = [
      idx.token_in,
      idx.token_out,
      ...(idx.intermediate_tokens ?? []),
      ...idx.hops.flatMap((h) => [h.offer_token, h.ask_token]),
    ]
    if (shouldRejectGemBridgeQuote(offerCw20, invoice.invoiceToken, hopTokens)) {
      return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
    }
    if (!sameTokenId(idx.token_in, offerCw20) || !sameTokenId(idx.token_out, invoice.invoiceToken)) {
      return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
    }
    ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
  } catch {
    return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
  }

  if (ops.length === 0) return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
  const pathErr = assertOperationsMatchPayPath(ops, offerCw20, invoice.invoiceToken)
  if (pathErr) return { status: 'unavailable', disableReason: pathErr }

  let reverseOffer: string
  try {
    reverseOffer = (await deps.reverseSimulate(invoice.invoiceAmount, ops)).amount
  } catch {
    return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
  }
  if (!isPositiveRawAmount(reverseOffer)) {
    return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
  }

  let offer = reverseOffer
  try {
    const bumped = await bumpOfferUntilInvoice({
      offer,
      operations: ops,
      invoiceAmount: invoice.invoiceAmount,
      trader: input.trader,
      forwardSimulate: deps.forwardSimulate,
    })
    if (!bumped) return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
    offer = bumped
  } catch {
    return { status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE }
  }

  const maxCw20 = applySlippagePercentCeiling(offer, input.slippagePercent)
  if (!maxCw20) return { status: 'unavailable', disableReason: PAY_INVOICE_INVALID_INVOICE }

  let payRaw = maxCw20
  let cw20SendAmount = maxCw20
  let wrap: PayInvoiceQuoteOk['wrap']
  if (wrapDenom) {
    const gross = wrapGrossForNetCw20(BigInt(maxCw20), input.wrapFeeBps ?? 0)
    payRaw = gross.toString()
    cw20SendAmount = netAfterWrapMapperFee(gross, input.wrapFeeBps ?? 0).toString()
    wrap = { denom: wrapDenom, grossNative: payRaw }
  }

  try {
    payRaw = await applyTaxDebit(payRaw, input.taxPreviewExtraDebit)
  } catch {
    return { status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT }
  }

  if (balanceShort(input.payTokenBalance, payRaw)) {
    return { status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT }
  }

  const display = (id: string) => id.slice(0, 6)
  return {
    status: 'ok',
    kind: wrap ? 'wrap_routed' : 'routed',
    payToken,
    offerCw20,
    payRaw,
    cw20SendAmount,
    maxIn: payRaw,
    minInvoiceOut: invoice.invoiceAmount,
    operations: ops,
    wrap,
    routeLabel: routeLabelFromOps(ops, display),
    hopCount: ops.length,
  }
}

export function assertOperationsMatchPayPath(
  operations: SwapOperation[],
  offerCw20: string,
  invoiceToken: string
): string | null {
  if (operations.length === 0) return PAY_INVOICE_NO_ROUTE
  const first = operations[0]?.terra_swap.offer_asset_info
  const last = operations[operations.length - 1]?.terra_swap.ask_asset_info
  if (!first || !last) return PAY_INVOICE_NO_ROUTE
  if ('native_token' in first || operations.some((op) => 'native_token' in op.terra_swap.offer_asset_info)) {
    return PAY_INVOICE_NO_ROUTE
  }
  if (!sameTokenId(assetInfoLabel(first), offerCw20)) return PAY_INVOICE_NO_ROUTE
  if (!sameTokenId(assetInfoLabel(last), invoiceToken)) return PAY_INVOICE_NO_ROUTE
  for (let i = 1; i < operations.length; i++) {
    const prevAsk = assetInfoLabel(operations[i - 1]!.terra_swap.ask_asset_info)
    const nextOffer = assetInfoLabel(operations[i]!.terra_swap.offer_asset_info)
    if (!sameTokenId(prevAsk, nextOffer)) return PAY_INVOICE_NO_ROUTE
  }
  return null
}

export class PayInvoiceBuilderError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'PayInvoiceBuilderError'
  }
}

export type BuildPayInvoiceMsgsInput = {
  invoice: Invoice
  quote: PayInvoiceQuoteOk
  walletAddress: string
  slippagePercent: number
}

function decodeSendHook(msg: Record<string, unknown>): Record<string, unknown> | null {
  const send = msg.send as { msg?: string } | undefined
  if (!send?.msg) return null
  try {
    return JSON.parse(atob(send.msg)) as Record<string, unknown>
  } catch {
    return null
  }
}

/**
 * Fail-closed checks for the execute payload (attack plan: min-receive, overpay, fake route).
 */
export function assertPayInvoiceMsgs(msgs: TerraExecuteContractEntry[], input: BuildPayInvoiceMsgsInput): void {
  const { invoice, quote, walletAddress } = input
  if (msgs.length === 0) throw new PayInvoiceBuilderError('empty settlement')
  if (msgs.some((m) => 'increase_allowance' in m.msg || 'decrease_allowance' in m.msg)) {
    throw new PayInvoiceBuilderError('allowance is not part of invoice pay')
  }

  const payeeSend = msgs[msgs.length - 1]
  if (!payeeSend || !sameTokenId(payeeSend.contract, invoice.invoiceToken)) {
    throw new PayInvoiceBuilderError('last msg must Send invoice token')
  }
  const payeeInner = payeeSend.msg.send as { contract?: string; amount?: string; msg?: string } | undefined
  if (!payeeInner) throw new PayInvoiceBuilderError('last msg must be CW20 Send')
  if (!sameTokenId(payeeInner.contract ?? '', invoice.payee)) {
    throw new PayInvoiceBuilderError('payee mismatch')
  }
  if (payeeInner.amount !== invoice.invoiceAmount) {
    throw new PayInvoiceBuilderError('payee amount must equal invoice')
  }
  if ((invoice.hookMsg || '') !== (payeeInner.msg || '')) {
    throw new PayInvoiceBuilderError('hook mismatch')
  }

  if (quote.kind === 'direct') {
    if (msgs.length !== 1) throw new PayInvoiceBuilderError('direct pay is a single Send')
    return
  }

  const swapSend = msgs.find((m) => {
    const inner = decodeSendHook(m.msg)
    return inner != null && 'execute_swap_operations' in inner
  })
  if (!swapSend) throw new PayInvoiceBuilderError('routed pay requires execute_swap_operations')
  const inner = decodeSendHook(swapSend.msg)!
  const exec = inner.execute_swap_operations as {
    minimum_receive?: string
    to?: string
    operations?: unknown[]
  }
  if (!exec.minimum_receive || BigInt(exec.minimum_receive) < BigInt(invoice.invoiceAmount)) {
    throw new PayInvoiceBuilderError('minimum_receive must be ≥ invoice')
  }
  if (exec.to && sameTokenId(exec.to, invoice.payee)) {
    throw new PayInvoiceBuilderError('swap to must not be the payee')
  }
  if (exec.to && !sameTokenId(exec.to, walletAddress)) {
    throw new PayInvoiceBuilderError('swap to must be the user')
  }
  const sendAmt = (swapSend.msg.send as { amount?: string }).amount
  if (sendAmt !== quote.cw20SendAmount) {
    throw new PayInvoiceBuilderError('router Send amount must equal quoted cw20SendAmount')
  }
  if (!exec.operations?.length) throw new PayInvoiceBuilderError('operations required')
}

export function buildPayInvoiceMsgs(input: BuildPayInvoiceMsgsInput): TerraExecuteContractEntry[] {
  const invalid = validateInvoice(input.invoice)
  if (invalid) throw new PayInvoiceBuilderError(invalid)
  if (!isValidTerraBech32Address(input.walletAddress)) {
    throw new PayInvoiceBuilderError('invalid wallet')
  }
  if (input.quote.minInvoiceOut !== input.invoice.invoiceAmount) {
    throw new PayInvoiceBuilderError('minInvoiceOut must equal invoice')
  }
  if (!ROUTER_CONTRACT_ADDRESS && input.quote.kind !== 'direct') {
    throw new PayInvoiceBuilderError('router unset')
  }

  const payeeSend: TerraExecuteContractEntry = {
    contract: input.invoice.invoiceToken,
    msg: {
      send: {
        contract: input.invoice.payee,
        amount: input.invoice.invoiceAmount,
        msg: input.invoice.hookMsg || '',
      },
    },
  }

  if (input.quote.kind === 'direct') {
    const msgs = [payeeSend]
    assertPayInvoiceMsgs(msgs, input)
    return msgs
  }

  const pathErr = assertOperationsMatchPayPath(
    input.quote.operations,
    input.quote.offerCw20,
    input.invoice.invoiceToken
  )
  if (pathErr) throw new PayInvoiceBuilderError(pathErr)

  const maxSpread = slippageToMaxSpread(input.slippagePercent)
  const swapHook = {
    execute_swap_operations: {
      operations: input.quote.operations.map((op) => ({
        terra_swap: serializeTerraSwap(op.terra_swap),
      })),
      max_spread: maxSpread,
      minimum_receive: input.invoice.invoiceAmount,
      to: input.walletAddress,
    },
  }

  const routerSend: TerraExecuteContractEntry = {
    contract: input.quote.offerCw20,
    msg: {
      send: {
        contract: ROUTER_CONTRACT_ADDRESS,
        amount: input.quote.cw20SendAmount,
        msg: btoa(JSON.stringify(swapHook)),
      },
    },
  }

  const msgs: TerraExecuteContractEntry[] = []
  if (input.quote.kind === 'wrap_routed') {
    if (!input.quote.wrap || !TREASURY_CONTRACT_ADDRESS) {
      throw new PayInvoiceBuilderError(PAY_INVOICE_WRAP_UNAVAILABLE)
    }
    if (!sameTokenId(input.quote.wrap.denom, input.quote.payToken)) {
      throw new PayInvoiceBuilderError('wrap denom mismatch')
    }
    msgs.push({
      contract: TREASURY_CONTRACT_ADDRESS,
      msg: { wrap_deposit: {} },
      coins: [{ denom: input.quote.wrap.denom, amount: input.quote.wrap.grossNative }],
    })
  }
  msgs.push(routerSend, payeeSend)
  assertPayInvoiceMsgs(msgs, input)
  return msgs
}

export function payInvoiceBroadcastCount(msgs: TerraExecuteContractEntry[]): 1 {
  if (msgs.length === 0) throw new PayInvoiceBuilderError('empty settlement')
  return 1
}

/** Token ids offered in the pay picker (catalog + wrap natives). Invoice token stays selectable. */
export function payInvoicePickerTokens(catalogTokens: string[]): string[] {
  const ids = [...catalogTokens]
  if (isNativeWrapEnabled()) {
    if (!ids.includes('uluna')) ids.unshift('uluna')
    if (!ids.includes('uusd')) ids.splice(ids.includes('uluna') ? 1 : 0, 0, 'uusd')
  }
  return [...new Set(ids.filter(Boolean))]
}

export function defaultPayToken(input: {
  invoiceToken: string
  invoiceAmount: string
  pickerTokens: string[]
  balances: Record<string, string>
}): string {
  const invoiceBal = input.balances[input.invoiceToken]
  if (invoiceBal && BigInt(invoiceBal) >= BigInt(input.invoiceAmount)) {
    return input.invoiceToken
  }
  if (input.pickerTokens.includes(input.invoiceToken)) return input.invoiceToken
  return input.pickerTokens[0] ?? input.invoiceToken
}

/** CosmWasm `token` AssetInfo helper for tests / callers that build ops. */
export function invoiceTokenAsset(addr: string) {
  return tokenAssetInfo(addr)
}
