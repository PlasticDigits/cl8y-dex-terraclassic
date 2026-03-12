import type {
  IndexerPair,
  IndexerCandle,
  IndexerTrade,
  IndexerPairStats,
  IndexerOverview,
  IndexerTrader,
  IndexerPosition,
} from '@/types'

const INDEXER_URL = import.meta.env.VITE_INDEXER_URL || 'http://localhost:3001'
const FETCH_TIMEOUT_MS = 15_000
const MAX_RETRIES = 1

async function fetchJson<T>(path: string): Promise<T> {
  let lastError: Error | undefined
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS)
    try {
      const resp = await fetch(`${INDEXER_URL}${path}`, { signal: controller.signal })
      if (!resp.ok) {
        throw new Error(`Indexer API error: ${resp.status} ${resp.statusText}`)
      }
      const text = await resp.text()
      try {
        return JSON.parse(text) as T
      } catch {
        throw new Error(`Indexer returned invalid JSON for ${path}`)
      }
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err))
      const isRetryable = lastError.name === 'AbortError'
        || lastError.message.includes('Failed to fetch')
        || lastError.message.includes('NetworkError')
      if (!isRetryable || attempt >= MAX_RETRIES) throw lastError
    } finally {
      clearTimeout(timer)
    }
  }
  throw lastError!
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
  sort = 'total_volume',
  limit = 50
): Promise<IndexerTrader[]> {
  const params = new URLSearchParams({ sort, limit: limit.toString() })
  return fetchJson<IndexerTrader[]>(`/api/v1/traders/leaderboard?${params}`)
}

/** Get trader's open positions with P&L. */
export async function getTraderPositions(
  address: string
): Promise<IndexerPosition[]> {
  return fetchJson<IndexerPosition[]>(`/api/v1/traders/${address}/positions`)
}
