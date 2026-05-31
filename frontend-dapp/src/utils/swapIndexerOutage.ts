import { detectMarketDataOutage, type MarketDataOutageQuerySlice } from '@/utils/marketDataOutage'

export type SwapSimOutageSlice = {
  /** Indexer HTTP failed during quote; pool-only LCD fallback may still succeed. */
  indexerTransportFailed?: boolean
}

/** Swap page: sim query transport failure or explicit indexer failure during quote (GitLab #241). */
export function detectSwapIndexerOutage(
  simQuery: MarketDataOutageQuerySlice,
  simData?: SwapSimOutageSlice | null
): boolean {
  return detectMarketDataOutage(simQuery) || !!simData?.indexerTransportFailed
}
