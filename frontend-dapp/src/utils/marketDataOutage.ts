import { isIndexerUnavailableError } from '@/utils/indexerErrors'

export type MarketDataOutageQuerySlice = {
  isError: boolean
  error: unknown
}

/** True when any indexer-backed query failed because the market data HTTP API is down. */
export function detectMarketDataOutage(...queries: MarketDataOutageQuerySlice[]): boolean {
  return queries.some((q) => q.isError && isIndexerUnavailableError(q.error))
}
