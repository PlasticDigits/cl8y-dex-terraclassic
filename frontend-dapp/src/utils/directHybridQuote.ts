import { postRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import { simulateHybridSwap, type QuoteTraderOptions } from '@/services/terraclassic/pair'
import { preflightSwapRouteSpread, type SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import type { SwapOperation } from '@/services/terraclassic/router'
import type { AssetInfo, HybridSwapParams, IndexerRouteQuoteKind } from '@/types'

export type DirectHybridQuoteResult = {
  return_amount: string
  spread_amount: string
  commission_amount: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  routePreflight?: SwapRoutePreflightSpread
  routeSlippagePercent?: string
  spotAmountOut?: string
}

/** Human-readable quote source for direct hybrid quotes (GitLab #418). */
export function quoteDisclosureForIndexerKind(kind: IndexerRouteQuoteKind | undefined): string {
  switch (kind) {
    case 'indexer_hybrid_lcd_degraded':
      return 'Indexer hybrid route (LCD) — one or more hops fell back to pool-only on the indexer.'
    case 'indexer_hybrid_lcd':
    case 'indexer_hybrid_db':
      return 'Indexer-optimized hybrid splits · quoted via your wallet LCD simulation (matches submit shape).'
    case 'indexer_pool_lcd':
      return 'Indexer route (pool-only legs) · quoted via your wallet LCD simulation.'
    case 'indexer_route_only':
      return 'Indexer-solved route · no aggregate router estimate (simulation unavailable).'
    default:
      return 'Direct pair · hybrid_simulation (Pattern C).'
  }
}

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
      return {
        return_amount: idx.estimated_amount_out,
        spread_amount: '0',
        commission_amount: '0',
        indexerQuoteKind: idx.quote_kind,
        indexerOperations: ops.length > 0 ? ops : undefined,
        routePreflight,
        routeSlippagePercent: idx.slippage_percent,
        spotAmountOut: idx.spot_amount_out,
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
