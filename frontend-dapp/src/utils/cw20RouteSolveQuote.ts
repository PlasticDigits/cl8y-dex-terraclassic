import { assertRouteTaxFunding, ROUTE_TAX_FUNDING_UNAVAILABLE } from '@/services/terraclassic/routeTaxFunding'
import { getPair } from '@/services/terraclassic/factory'
import { getRouteSolve, getRouteSolveExactOutput } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import type { QuoteTraderOptions } from '@/services/terraclassic/pair'
import { simulateMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import {
  enrichSwapOperationsWithHopMinReturns,
  preflightSwapRouteSpread,
  type SwapRoutePreflightSpread,
} from '@/services/terraclassic/swapRoutePreflight'
import type { IndexerRouteQuoteKind, IndexerRouteSolveResponse, PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import { reconcileSwapRouteIntermediateTokens } from '@/utils/swapRouteDisplay'
import { resolveRouteSlippagePercent } from '@/utils/swapRouteSlippage'
import { shouldRejectGemBridgeQuote } from '@/utils/pairCatalogRank'
import { displayReceiveNet } from '@/utils/communityTaxNetOut'
import {
  assertHop0DeclaredHybridPartitionsOffer,
  stripAllDeclaredHybrid,
  stripInteriorDeclaredHybrid,
} from '@/utils/hybridHopOfferPartition'

/**
 * Wallet-authoritative CW20 quote from indexer `GET /route/solve` (global best-execution hybrid).
 * Shared by Swap and Trade market (GitLab #501 / always-on #596).
 *
 * Invariants:
 * - Receive **display** is wallet sim minus catalog buy split when indexer `buy_tax_bps` > 0 (#615).
 * - Submit `min_return` uses pre-tax wallet sim (`executeAmountOut`), not the net display.
 * - Submit must use returned `indexerOperations` (hop-0 `hybrid` after Policy A strip; #1280).
 * - Wrap/native execute hops never copy `hybrid` (#1264 / **H596-7** / #1218). Pass
 *   `omitAllHybrid: true` on wrap-enter / unwrap-exit so wallet sim matches pool-only submit.
 * - Returns `null` when token_in/out mismatch the request (caller falls back).
 * - Throws on indexer/wallet failure (caller catches for pool-only / Advanced fallback).
 */
export type Cw20RouteSolveQuote = {
  return_amount: string
  taxFundingVersion?: 1
  /** Pre-tax wallet sim for `min_return` when `return_amount` is post-buy-split (#615). */
  executeAmountOut?: string
  spread_amount: string
  commission_amount: string
  routeSlippagePercent?: string
  spotAmountOut?: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerPairs?: PairInfo[]
  indexerOperations: SwapOperation[]
  indexerIntermediateTokens?: string[]
  indexerRouteIntermediateReconciled?: boolean
  routePreflight?: SwapRoutePreflightSpread
}

type RouteSolveQuoteInput = {
  fromToken: string
  toToken: string
  simRaw: string
  maxMakerFills: number
  slippageTolerancePercent: number
  maxSpreadStr: string
  quoteTrader?: QuoteTraderOptions
  signal?: AbortSignal
  /** Strip hop-0 hybrid too so wrap-enter quote = pool-only execute (#1218 / **H596-7**). */
  omitAllHybrid?: boolean
}

export async function quoteCw20ViaRouteSolve(input: RouteSolveQuoteInput): Promise<Cw20RouteSolveQuote | null> {
  const idx = await getRouteSolve(input.fromToken, input.toToken, input.simRaw, {
    maxMakerFills: input.maxMakerFills,
    trader: input.quoteTrader?.trader,
    sender: input.quoteTrader?.sender,
    signal: input.signal,
  })
  return validateRouteSolveQuote(input, idx)
}

async function validateRouteSolveQuote(
  input: RouteSolveQuoteInput,
  idx: IndexerRouteSolveResponse
): Promise<Cw20RouteSolveQuote | null> {
  const {
    fromToken,
    toToken,
    simRaw,
    slippageTolerancePercent,
    maxSpreadStr,
    quoteTrader: baseQuoteTrader,
    omitAllHybrid,
  } = input

  input.signal?.throwIfAborted()

  const tin = idx.token_in.trim().toLowerCase()
  const tout = idx.token_out.trim().toLowerCase()
  if (tin !== fromToken.trim().toLowerCase() || tout !== toToken.trim().toLowerCase()) {
    return null
  }

  const hopTokens = [
    idx.token_in,
    idx.token_out,
    ...(idx.intermediate_tokens ?? []),
    ...idx.hops.flatMap((h) => [h.offer_token, h.ask_token]),
  ]
  if (shouldRejectGemBridgeQuote(fromToken, toToken, hopTokens)) {
    return null
  }

  const mapped = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
  let currentToken = tin
  for (const [i, op] of mapped.entries()) {
    const offer = assetInfoLabel(op.terra_swap.offer_asset_info).trim().toLowerCase()
    const ask = assetInfoLabel(op.terra_swap.ask_asset_info).trim().toLowerCase()
    if (
      offer !== currentToken ||
      idx.hops[i].offer_token.trim().toLowerCase() !== offer ||
      idx.hops[i].ask_token.trim().toLowerCase() !== ask
    ) {
      throw new Error('Route operations do not match the quoted token path. Refresh the quote.')
    }
    currentToken = ask
  }
  if (currentToken !== tout) throw new Error('Route receive token does not match the quote. Refresh the quote.')
  const ops = omitAllHybrid ? stripAllDeclaredHybrid(mapped) : stripInteriorDeclaredHybrid(mapped)
  const fundingVersion = await assertRouteTaxFunding(ops, input.signal)
  if ((idx.tax_funding_version === 1 || idx.tax_kind === 'route_local_v1') && fundingVersion !== 1) {
    throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
  }
  const fundedTrader = { ...baseQuoteTrader, taxFundingVersion: fundingVersion }
  const quoteTrader = input.signal ? { ...fundedTrader, signal: input.signal } : fundedTrader
  if (!omitAllHybrid && fundingVersion !== 1) {
    assertHop0DeclaredHybridPartitionsOffer(ops, simRaw)
  }
  const opsForQuote = await enrichSwapOperationsWithHopMinReturns(ops, simRaw, slippageTolerancePercent, quoteTrader)
  const result = await simulateMultiHopSwap(simRaw, opsForQuote, quoteTrader)
  const routePreflight = await preflightSwapRouteSpread(opsForQuote, simRaw, maxSpreadStr, quoteTrader)
  const intermediates =
    idx.intermediate_tokens?.length === idx.hops.length + 1
      ? idx.intermediate_tokens
      : [idx.token_in, ...idx.hops.map((h) => h.ask_token)]
  const { tokens: reconciledIntermediates, mismatch: routeIntermediateMismatch } = reconcileSwapRouteIntermediateTokens(
    opsForQuote,
    intermediates
  )
  if (routeIntermediateMismatch) {
    console.warn(
      '[route-solve] indexer intermediate_tokens disagreed with router_operations; reconciled route display to ops path'
    )
  }

  const rawWallet = result.amount
  const buyBps = fundingVersion === 1 ? 0 : (idx.buy_tax_bps ?? 0)
  const displayNet = displayReceiveNet(rawWallet, buyBps)

  return {
    return_amount: displayNet,
    taxFundingVersion: fundingVersion,
    executeAmountOut: displayNet !== rawWallet ? rawWallet : undefined,
    spread_amount: '0',
    commission_amount: '0',
    routeSlippagePercent: resolveRouteSlippagePercent(result.amount, idx.spot_amount_out, idx.slippage_percent),
    spotAmountOut: idx.spot_amount_out,
    indexerQuoteKind: idx.quote_kind,
    indexerOperations: opsForQuote,
    indexerIntermediateTokens: reconciledIntermediates,
    indexerRouteIntermediateReconciled: routeIntermediateMismatch,
    routePreflight,
  }
}

function isPositiveUint128Raw(value: unknown): value is string {
  return (
    typeof value === 'string' &&
    value.length <= 39 &&
    /^\d+$/.test(value) &&
    BigInt(value) > 0n &&
    BigInt(value) <= (1n << 128n) - 1n
  )
}

/** Server selection + authoritative wallet validation of that exact offer/path. No browser search. */
export async function quoteCw20ExactOutputViaRouteSolve(
  input: Omit<RouteSolveQuoteInput, 'simRaw'> & { targetRaw: string }
): Promise<Cw20RouteSolveQuote & { offerAmount: string }> {
  const idx = await getRouteSolveExactOutput(input.fromToken, input.toToken, input.targetRaw, {
    trader: input.quoteTrader?.trader,
    sender: input.quoteTrader?.sender,
    signal: input.signal,
  })
  if (
    idx.sizing_mode !== 'pool_only_exact_output' ||
    idx.target_amount_out !== input.targetRaw ||
    !isPositiveUint128Raw(idx.required_amount_in) ||
    idx.fidelity_check !== 'passed'
  ) {
    throw new Error('Receive sizing returned an invalid or stale offer. Retry the quote.')
  }
  // Exact sizing must retain the global route reference used by the 5/30/99 protections.
  if (!isPositiveUint128Raw(idx.spot_amount_out)) {
    throw new Error('Receive sizing fair price reference is unavailable or invalid. Retry the quote.')
  }
  if (!idx.hops.length || idx.hops.length > 4) throw new Error('Invalid receive route hop count')
  if (
    !Array.isArray(idx.router_operations) ||
    idx.router_operations.some((op) => {
      const ts = (op as { terra_swap?: Record<string, unknown> } | null)?.terra_swap
      return (
        !ts ||
        ts.hybrid != null ||
        Object.keys(ts).some((key) => !['offer_asset_info', 'ask_asset_info', 'hybrid', 'min_return'].includes(key))
      )
    })
  )
    throw new Error('Unsupported receive route execution policy')
  const ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
  let token = input.fromToken
  for (const [i, op] of ops.entries()) {
    const hop = idx.hops[i]
    if (
      !('token' in op.terra_swap.offer_asset_info) ||
      !('token' in op.terra_swap.ask_asset_info) ||
      op.terra_swap.offer_asset_info.token.contract_addr !== token ||
      hop.offer_token !== token ||
      op.terra_swap.ask_asset_info.token.contract_addr !== hop.ask_token ||
      op.terra_swap.hybrid
    ) {
      throw new Error('Invalid or unsupported receive route operations')
    }
    token = hop.ask_token
  }
  if (token !== input.toToken) throw new Error('Invalid receive route endpoint')
  const indexerPairs: PairInfo[] = []
  for (const [i, op] of ops.entries()) {
    input.signal?.throwIfAborted()
    const pair = await getPair(
      [op.terra_swap.offer_asset_info, op.terra_swap.ask_asset_info],
      ...(input.signal ? ([input.signal] as const) : ([] as const))
    )
    if (pair.contract_addr !== idx.hops[i].pair) throw new Error('Receive route pair is not the listed factory pair')
    indexerPairs.push(pair)
  }
  const quote = await validateRouteSolveQuote({ ...input, simRaw: idx.required_amount_in, omitAllHybrid: true }, idx)
  if (!quote || BigInt(quote.return_amount) < BigInt(input.targetRaw)) {
    throw new Error('Fresh route output cannot satisfy the receive target. Retry the quote.')
  }
  return { ...quote, indexerPairs, offerAmount: idx.required_amount_in }
}
