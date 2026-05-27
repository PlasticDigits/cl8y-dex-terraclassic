import { isIndexerUnavailableError } from '@/utils/indexerErrors'

type OutageQuerySlice = {
  isError: boolean
  error: unknown
}

/** True when any trade workspace query failed because the indexer HTTP API is down. */
export function detectTradeIndexerOutage(...queries: OutageQuerySlice[]): boolean {
  return queries.some((q) => q.isError && isIndexerUnavailableError(q.error))
}
