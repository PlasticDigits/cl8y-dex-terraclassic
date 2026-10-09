import { maxSpreadToSlippagePercent } from '@/utils/slippageConstraint'
import type { QuoteTraderOptions } from '@/services/terraclassic/pair'
import type { AssetInfo, HybridSwapParams, IndexerRouteQuoteKind } from '@/types'
import type { Cw20RouteSolveQuote } from '@/utils/cw20RouteSolveQuote'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'

export type DirectHybridQuoteResult = Cw20RouteSolveQuote & { indexerAmountReconciled?: boolean }

/** Shown when direct-hybrid receive was reconciled from indexer to wallet sim (GitLab #471). */
export const DIRECT_HYBRID_AMOUNT_RECONCILED_COPY = 'Receive amount adjusted to match your wallet.'

/** Retail quote-source line for direct hybrid / market quote cards (GitLab #418, #414). */
export function quoteDisclosureForIndexerKind(kind: IndexerRouteQuoteKind | undefined): string {
  switch (kind) {
    case 'indexer_hybrid_lcd_degraded':
      return 'Estimated from limit book + pool routing. One or more hops used pool-only pricing.'
    case 'indexer_hybrid_lcd':
    case 'indexer_hybrid_db':
      return 'Estimated output from limit book + pool routing.'
    case 'indexer_pool_lcd':
      return 'Estimated output from pool routing.'
    case 'indexer_route_only':
      return 'Route found; estimated output unavailable — review carefully before submit.'
    default:
      return 'Estimated output from limit book + pool routing on this pair.'
  }
}

/** Pool-only direct pair quote (no limit book leg). */
export const POOL_ONLY_QUOTE_DISCLOSURE = 'Estimated output from pool-only pricing on this pair.'

/** Manual book splits require supported local executable metadata; no live simulation fallback. */
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
  if (BigInt(input.hybrid.book_input) > 0n) throw new Error('This book split is unavailable. Clear the book leg.')
  const quote = await quoteCw20ViaRouteSolve({
    ...input,
    maxMakerFills: input.hybrid.max_maker_fills,
    slippageTolerancePercent: maxSpreadToSlippagePercent(input.maxSpreadStr),
    signal: input.quoteTrader?.signal,
  })
  if (
    !quote ||
    quote.executionMetadata.hops.length !== 1 ||
    quote.executionMetadata.hops[0].pair !== input.pairAddress
  ) {
    throw new Error('This pair quote is unavailable. Refresh the quote.')
  }
  return quote
}
