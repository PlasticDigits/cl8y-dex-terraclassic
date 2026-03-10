import type {
  IndexerPair,
  IndexerCandle,
  IndexerTrade,
  IndexerPairStats,
  IndexerOverview,
  IndexerTrader,
} from '@/types'

const INDEXER_URL = import.meta.env.VITE_INDEXER_URL || 'http://localhost:3001'

async function fetchJson<T>(path: string): Promise<T> {
  const resp = await fetch(`${INDEXER_URL}${path}`)
  if (!resp.ok) {
    throw new Error(`Indexer API error: ${resp.status} ${resp.statusText}`)
  }
  return resp.json() as Promise<T>
}

/** Get all indexed pairs with enriched asset info. */
export async function getPairs(): Promise<IndexerPair[]> {
  return fetchJson<IndexerPair[]>('/api/v1/pairs')
}

/** Get OHLCV candles for a pair. */
export async function getCandles(
  pairAddr: string,
  interval = '1h',
  from?: string,
  to?: string,
  limit = 200
): Promise<IndexerCandle[]> {
  const params = new URLSearchParams({ interval, limit: limit.toString() })
  if (from) params.set('from', from)
  if (to) params.set('to', to)
  return fetchJson<IndexerCandle[]>(`/api/v1/pairs/${pairAddr}/candles?${params}`)
}

/** Get recent trades for a pair. */
export async function getTrades(
  pairAddr: string,
  limit = 50,
  before?: number
): Promise<IndexerTrade[]> {
  const params = new URLSearchParams({ limit: limit.toString() })
  if (before) params.set('before', before.toString())
  return fetchJson<IndexerTrade[]>(`/api/v1/pairs/${pairAddr}/trades?${params}`)
}

/** Get 24h stats for a pair. */
export async function getPairStats(pairAddr: string): Promise<IndexerPairStats> {
  return fetchJson<IndexerPairStats>(`/api/v1/pairs/${pairAddr}/stats`)
}

/** Get global DEX overview stats. */
export async function getOverview(): Promise<IndexerOverview> {
  return fetchJson<IndexerOverview>('/api/v1/overview')
}

/** Get trader profile. */
export async function getTrader(address: string): Promise<IndexerTrader> {
  return fetchJson<IndexerTrader>(`/api/v1/traders/${address}`)
}

/** Get trader's historical trades. */
export async function getTraderTrades(
  address: string,
  limit = 50,
  before?: number
): Promise<IndexerTrade[]> {
  const params = new URLSearchParams({ limit: limit.toString() })
  if (before) params.set('before', before.toString())
  return fetchJson<IndexerTrade[]>(`/api/v1/traders/${address}/trades?${params}`)
}

/** Get trader leaderboard. */
export async function getLeaderboard(
  sort = 'volume_24h',
  limit = 50
): Promise<IndexerTrader[]> {
  const params = new URLSearchParams({ sort, limit: limit.toString() })
  return fetchJson<IndexerTrader[]>(`/api/v1/traders/leaderboard?${params}`)
}
