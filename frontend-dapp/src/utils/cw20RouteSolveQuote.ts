import { getRouteSolve, getRouteSolveExactOutput } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import type { QuoteTraderOptions } from '@/services/terraclassic/pair'
import type { SwapOperation } from '@/services/terraclassic/router'
import type { SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import type { IndexerRouteExecutionMetadata, IndexerRouteQuoteKind, IndexerRouteSolveResponse, PairInfo } from '@/types'
import { assetInfoLabel, getWrappedEquivalent, isNativeDenom } from '@/types'
import { isLocalQuoteRaw, LOCAL_ROUTE_UNAVAILABLE, validateLocalRouteQuote } from '@/utils/localRouteQuote'
import { shouldRejectGemBridgeQuote } from '@/utils/pairCatalogRank'
import { reconcileSwapRouteIntermediateTokens } from '@/utils/swapRouteDisplay'
import { resolveRouteSlippagePercent } from '@/utils/swapRouteSlippage'

/** Observation-pinned indexed quote. Missing executable metadata never starts a chain lookup. */
export type Cw20RouteSolveQuote = {
  return_amount: string
  minimumReceive: string
  executionMetadata: IndexerRouteExecutionMetadata
  executionOperations: string
  taxFundingVersion?: 1
  /** Legacy display compatibility; local output and per-hop floors are already net. */
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
  /** Request a pool-only plan; reject any unsupported hybrid instead of changing quoted operations. */
  omitAllHybrid?: boolean
}

export async function quoteCw20ViaRouteSolve(input: RouteSolveQuoteInput): Promise<Cw20RouteSolveQuote | null> {
  const idx = await getRouteSolve(input.fromToken, input.toToken, input.simRaw, {
    maxMakerFills: input.maxMakerFills,
    poolOnly: input.omitAllHybrid || isNativeDenom(input.fromToken) || isNativeDenom(input.toToken),
    maxSpread: input.maxSpreadStr,
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
  if (
    !idx ||
    typeof idx.token_in !== 'string' ||
    typeof idx.token_out !== 'string' ||
    !Array.isArray(idx.hops) ||
    !idx.hops.length ||
    idx.hops.length > 4 ||
    idx.hops.some(
      (h) => !h || typeof h.offer_token !== 'string' || typeof h.ask_token !== 'string' || typeof h.pair !== 'string'
    ) ||
    (idx.intermediate_tokens != null &&
      (!Array.isArray(idx.intermediate_tokens) || idx.intermediate_tokens.some((token) => typeof token !== 'string')))
  ) {
    throw new Error(LOCAL_ROUTE_UNAVAILABLE)
  }
  assertLocalOperationSchema(idx.router_operations)

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
  let currentToken = idx.execution_metadata?.wrapper?.wrap_input ? (getWrappedEquivalent(tin) ?? tin) : tin
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
  const routeOut = idx.execution_metadata?.wrapper?.unwrap_output ? (getWrappedEquivalent(tout) ?? tout) : tout
  if (currentToken !== routeOut) throw new Error('Route receive token does not match the quote. Refresh the quote.')
  if (omitAllHybrid && mapped.some((op) => op.terra_swap.hybrid != null)) throw new Error(LOCAL_ROUTE_UNAVAILABLE)
  const opsForQuote = mapped
  const { metadata, routePreflight } = validateLocalRouteQuote(idx, opsForQuote, {
    simRaw,
    maxSpreadStr,
    slippageTolerancePercent,
    trader: baseQuoteTrader?.trader,
    sender: baseQuoteTrader?.sender,
    maxMakerFills: input.maxMakerFills,
  })
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

  const displayNet = idx.estimated_amount_out!

  return {
    return_amount: displayNet,
    minimumReceive: metadata.minimum_receive,
    executionMetadata: metadata,
    executionOperations: JSON.stringify(opsForQuote),
    spread_amount: '0',
    commission_amount: '0',
    routeSlippagePercent: resolveRouteSlippagePercent(displayNet, idx.spot_amount_out, idx.slippage_percent),
    spotAmountOut: idx.spot_amount_out,
    indexerQuoteKind: idx.quote_kind,
    indexerOperations: opsForQuote,
    indexerIntermediateTokens: reconciledIntermediates,
    indexerRouteIntermediateReconciled: routeIntermediateMismatch,
    routePreflight,
  }
}

/** Reject unsupported controls before the compatibility mapper can omit them. */
function assertLocalOperationSchema(operations: unknown): void {
  const asset = (value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return false
    const a = value as Record<string, unknown>
    if (Object.keys(a).length !== 1 || !a.token || typeof a.token !== 'object' || Array.isArray(a.token)) return false
    const token = a.token as Record<string, unknown>
    return Object.keys(token).length === 1 && typeof token.contract_addr === 'string' && !!token.contract_addr
  }
  if (!Array.isArray(operations) || !operations.length || operations.length > 4)
    throw new Error(LOCAL_ROUTE_UNAVAILABLE)
  for (const op of operations) {
    if (!op || typeof op !== 'object' || Array.isArray(op) || Object.keys(op).length !== 1)
      throw new Error(LOCAL_ROUTE_UNAVAILABLE)
    const ts = (op as { terra_swap?: Record<string, unknown> }).terra_swap
    if (
      !ts ||
      typeof ts !== 'object' ||
      Array.isArray(ts) ||
      Object.keys(ts).some((key) => !['offer_asset_info', 'ask_asset_info', 'min_return', 'hybrid'].includes(key)) ||
      !asset(ts.offer_asset_info) ||
      !asset(ts.ask_asset_info) ||
      !isLocalQuoteRaw(ts.min_return)
    )
      throw new Error(LOCAL_ROUTE_UNAVAILABLE)
    if (ts.hybrid != null) {
      if (typeof ts.hybrid !== 'object' || Array.isArray(ts.hybrid)) throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      const h = ts.hybrid as Record<string, unknown>
      if (
        Object.keys(h).some(
          (key) => !['pool_input', 'book_input', 'max_maker_fills', 'book_start_hint'].includes(key)
        ) ||
        !isLocalQuoteRaw(h.pool_input, false) ||
        !isLocalQuoteRaw(h.book_input) ||
        !Number.isSafeInteger(h.max_maker_fills) ||
        h.book_start_hint != null
      )
        throw new Error(LOCAL_ROUTE_UNAVAILABLE)
    }
  }
}

/** Server selection + authoritative wallet validation of that exact offer/path. No browser search. */
export async function quoteCw20ExactOutputViaRouteSolve(
  input: Omit<RouteSolveQuoteInput, 'simRaw'> & { targetRaw: string }
): Promise<Cw20RouteSolveQuote & { offerAmount: string }> {
  const idx = await getRouteSolveExactOutput(input.fromToken, input.toToken, input.targetRaw, {
    trader: input.quoteTrader?.trader,
    sender: input.quoteTrader?.sender,
    maxSpread: input.maxSpreadStr,
    signal: input.signal,
  })
  if (
    !['pool_only_exact_output', 'local_pool_exact_output'].includes(idx.sizing_mode ?? '') ||
    idx.target_amount_out !== input.targetRaw ||
    !isLocalQuoteRaw(idx.required_amount_in)
  ) {
    throw new Error('Receive sizing returned an invalid or stale offer. Retry the quote.')
  }
  // Exact sizing must retain the global route reference used by the 5/30/99 protections.
  if (!isLocalQuoteRaw(idx.spot_amount_out)) {
    throw new Error('Receive sizing fair price reference is unavailable or invalid. Retry the quote.')
  }
  if (!Array.isArray(idx.hops) || !idx.hops.length || idx.hops.length > 4)
    throw new Error('Invalid receive route hop count')
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
  let token = idx.execution_metadata?.wrapper?.wrap_input
    ? (getWrappedEquivalent(input.fromToken) ?? input.fromToken)
    : input.fromToken
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
  if (token !== (idx.execution_metadata?.wrapper?.unwrap_output ? getWrappedEquivalent(input.toToken) : input.toToken))
    throw new Error('Invalid receive route endpoint')
  const quote = await validateRouteSolveQuote({ ...input, simRaw: idx.required_amount_in, omitAllHybrid: true }, idx)
  if (!quote || BigInt(quote.return_amount) < BigInt(input.targetRaw)) {
    throw new Error('Fresh route output cannot satisfy the receive target. Retry the quote.')
  }
  return {
    ...quote,
    minimumReceive: BigInt(quote.minimumReceive) > BigInt(input.targetRaw) ? quote.minimumReceive : input.targetRaw,
    offerAmount: idx.required_amount_in,
  }
}
