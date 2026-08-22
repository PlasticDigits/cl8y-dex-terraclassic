import { getRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import type { QuoteTraderOptions } from '@/services/terraclassic/pair'
import { simulateMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import {
  enrichSwapOperationsWithHopMinReturns,
  preflightSwapRouteSpread,
  type SwapRoutePreflightSpread,
} from '@/services/terraclassic/swapRoutePreflight'
import type { IndexerRouteQuoteKind } from '@/types'
import { reconcileSwapRouteIntermediateTokens } from '@/utils/swapRouteDisplay'
import { resolveRouteSlippagePercent } from '@/utils/swapRouteSlippage'
import { shouldRejectGemBridgeQuote } from '@/utils/pairCatalogRank'

/**
 * Wallet-authoritative CW20 quote from indexer `GET /route/solve` (global best-execution hybrid).
 * Shared by Swap and Trade market (GitLab #501 / always-on #596).
 *
 * Invariants:
 * - Receive amount comes from wallet `simulate_swap_operations` (or single-hop equivalent), not indexer `estimated_amount_out`.
 * - Submit must use returned `indexerOperations` (including per-hop `hybrid`) via `hybridFromSingleHopIndexerOps` / router execute.
 * - Returns `null` when token_in/out mismatch the request (caller falls back).
 * - Throws on indexer/wallet failure (caller catches for pool-only / Advanced fallback).
 */
export type Cw20RouteSolveQuote = {
  return_amount: string
  spread_amount: string
  commission_amount: string
  routeSlippagePercent?: string
  spotAmountOut?: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations: SwapOperation[]
  indexerIntermediateTokens?: string[]
  indexerRouteIntermediateReconciled?: boolean
  routePreflight?: SwapRoutePreflightSpread
}

export async function quoteCw20ViaRouteSolve(input: {
  fromToken: string
  toToken: string
  simRaw: string
  maxMakerFills: number
  slippageTolerancePercent: number
  maxSpreadStr: string
  quoteTrader?: QuoteTraderOptions
  signal?: AbortSignal
}): Promise<Cw20RouteSolveQuote | null> {
  const { fromToken, toToken, simRaw, maxMakerFills, slippageTolerancePercent, maxSpreadStr, quoteTrader, signal } =
    input

  const idx = await getRouteSolve(fromToken, toToken, simRaw, {
    maxMakerFills,
    trader: quoteTrader?.trader,
    signal,
  })
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

  const ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
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

  return {
    return_amount: result.amount,
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
