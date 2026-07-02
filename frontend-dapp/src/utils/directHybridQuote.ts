import { postRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import { simulateHybridSwap, type QuoteTraderOptions } from '@/services/terraclassic/pair'
import { preflightSwapRouteSpread, type SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import type { SwapOperation } from '@/services/terraclassic/router'
import type { AssetInfo, HybridSwapParams, IndexerRouteQuoteKind } from '@/types'
import { indexerWalletAmountMismatch, resolveRouteSlippagePercent } from '@/utils/swapRouteSlippage'

export type DirectHybridQuoteResult = {
  return_amount: string
  spread_amount: string
  commission_amount: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  routePreflight?: SwapRoutePreflightSpread
  routeSlippagePercent?: string
  spotAmountOut?: string
  /** Indexer `estimated_amount_out` disagreed with wallet `hybrid_simulation`; display uses wallet (GitLab #471). */
  indexerAmountReconciled?: boolean
}

/** Shown when direct-hybrid receive was reconciled from indexer to wallet sim (GitLab #471). */
export const DIRECT_HYBRID_AMOUNT_RECONCILED_COPY = 'Receive amount adjusted to match your wallet.'

/** Retail quote-source line for direct hybrid / market quote cards (GitLab #418, #414). */
export function quoteDisclosureForIndexerKind(kind: IndexerRouteQuoteKind | undefined): string {
  switch (kind) {
    case 'indexer_hybrid_lcd_degraded':
      return 'Estimated from limit book + pool routing. One or more hops used pool-only pricing.'
    case 'indexer_hybrid_lcd':
    case 'indexer_hybrid_db':
      return 'Estimated output from limit book + pool routing, checked against your wallet before submit.'
    case 'indexer_pool_lcd':
      return 'Estimated output from pool routing, checked against your wallet before submit.'
    case 'indexer_route_only':
      return 'Route found; estimated output unavailable — review carefully before submit.'
    default:
      return 'Estimated output from limit book + pool routing on this pair.'
  }
}

/** Pool-only direct pair quote (no limit book leg). */
export const POOL_ONLY_QUOTE_DISCLOSURE = 'Estimated output from pool-only pricing on this pair.'

/**
 * Quote a direct CW20 swap with hybrid params aligned to submit.
 * Tries indexer `POST /route/solve` first, then wallet LCD `hybrid_simulation`.
 * Never returns a pool-only quote when `hybrid` has a positive book leg (GitLab #418 / L8).
 */
export async function quoteDirectHybridSwap(input: {
  pairAddress: string
  fromToken: string
  toToken: string
  offerAssetInfo: AssetInfo
  askAssetInfo: AssetInfo
  simRaw: string
  hybrid: HybridSwapParams
  maxSpreadStr: string
  quoteTrader?: QuoteTraderOptions
}): Promise<DirectHybridQuoteResult> {
  const { pairAddress, fromToken, toToken, offerAssetInfo, askAssetInfo, simRaw, hybrid, maxSpreadStr, quoteTrader } =
    input

  let lastError: unknown
  try {
    const idx = await postRouteSolve(fromToken, toToken, simRaw, [hybrid], quoteTrader)
    if (idx.estimated_amount_out?.trim()) {
      const ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
      const routePreflight =
        ops.length > 0 ? await preflightSwapRouteSpread(ops, simRaw, maxSpreadStr, quoteTrader) : undefined
      // GitLab #471: reconcile receive + slippage to wallet sim (same as multi-hop indexer branch).
      const walletSim = await simulateHybridSwap(pairAddress, offerAssetInfo, simRaw, hybrid, quoteTrader)
      const indexerAmount = idx.estimated_amount_out.trim()
      const walletAmount = walletSim.return_amount
      const amountMismatch = indexerWalletAmountMismatch(indexerAmount, walletAmount)
      if (amountMismatch) {
        console.warn(
          '[directHybridQuote] indexer estimated_amount_out disagreed with wallet hybrid_simulation; reconciled receive to wallet amount'
        )
      }
      return {
        return_amount: walletAmount,
        spread_amount: walletSim.spread_amount,
        commission_amount: walletSim.commission_amount,
        indexerQuoteKind: idx.quote_kind,
        indexerOperations: ops.length > 0 ? ops : undefined,
        routePreflight,
        routeSlippagePercent: resolveRouteSlippagePercent(walletAmount, idx.spot_amount_out, idx.slippage_percent),
        spotAmountOut: idx.spot_amount_out,
        indexerAmountReconciled: amountMismatch,
      }
    }
  } catch (e) {
    lastError = e
  }

  try {
    const sim = await simulateHybridSwap(pairAddress, offerAssetInfo, simRaw, hybrid, quoteTrader)
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: offerAssetInfo,
          ask_asset_info: askAssetInfo,
          hybrid,
        },
      },
    ]
    const routePreflight = await preflightSwapRouteSpread(ops, simRaw, maxSpreadStr, quoteTrader)
    return {
      return_amount: sim.return_amount,
      spread_amount: sim.spread_amount,
      commission_amount: sim.commission_amount,
      routePreflight,
    }
  } catch (e) {
    lastError = e
  }

  throw lastError instanceof Error ? lastError : new Error('Hybrid quote unavailable')
}
