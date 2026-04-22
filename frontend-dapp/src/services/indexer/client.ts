import type {
  IndexerPair,
  IndexerPairsListResponse,
  IndexerPairSort,
  IndexerCandle,
  IndexerTrade,
  IndexerPairStats,
  IndexerOverview,
  IndexerTrader,
  IndexerPosition,
  IndexerToken,
  IndexerTokenDetail,
  IndexerHookEvent,
  IndexerOraclePriceResponse,
  IndexerOracleHistoryResponse,
  IndexerHybridHopInput,
  IndexerRouteSolveResponse,
  IndexerLimitFill,
  IndexerLiquidityEvent,
  IndexerLimitPlacement,
  IndexerLimitCancellation,
  IndexerOrderBookHeadResponse,
  IndexerLimitBookShallowResponse,
} from '@/types'

export const INDEXER_URL = import.meta.env.VITE_INDEXER_URL || 'http://127.0.0.1:3001'
const FETCH_TIMEOUT_MS = 15_000
const MAX_RETRIES = 1

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  let lastError: Error | undefined
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS)
    try {
      const resp = await fetch(`${INDEXER_URL}${path}`, {
        ...init,
        signal: controller.signal,
      })
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
      const isRetryable =
        lastError.name === 'AbortError' ||
        lastError.message.includes('Failed to fetch') ||
        lastError.message.includes('NetworkError')
      if (!isRetryable || attempt >= MAX_RETRIES) throw lastError
    } finally {
      clearTimeout(timer)
    }
  }
  throw lastError!
}

async function fetchJsonPost<T>(path: string, body: unknown): Promise<T> {
  return fetchJson<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
  })
}

export interface GetPairsParams {
  limit?: number
  offset?: number
  /** Search pair address, symbols, contracts, denoms */
  q?: string
  /** Exact CW20 contract or native denom — pairs that include this token */
  asset?: string
  sort?: IndexerPairSort
  order?: 'asc' | 'desc'
}

/** Paginated pair list from the indexer (sort, filter, search). */
export async function getPairs(params?: GetPairsParams): Promise<IndexerPairsListResponse> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.offset != null) sp.set('offset', String(params.offset))
  if (params?.q?.trim()) sp.set('q', params.q.trim())
  if (params?.asset?.trim()) sp.set('asset', params.asset.trim())
  if (params?.sort) sp.set('sort', params.sort)
  if (params?.order) sp.set('order', params.order)
  const qs = sp.toString()
  return fetchJson<IndexerPairsListResponse>(`/api/v1/pairs${qs ? `?${qs}` : ''}`)
}

/** Single pair metadata from `GET /api/v1/pairs/{addr}`. */
export async function getPair(pairAddr: string): Promise<IndexerPair> {
  return fetchJson<IndexerPair>(`/api/v1/pairs/${pairAddr}`)
}

/** Load up to `maxPairs` by paging the indexer (e.g. chart pair selector). */
export async function getAllPairsPaged(maxPairs = 5000, pageSize = 100): Promise<IndexerPair[]> {
  const out: IndexerPair[] = []
  let offset = 0
  while (out.length < maxPairs) {
    const page = await getPairs({ limit: pageSize, offset, sort: 'symbol', order: 'asc' })
    out.push(...page.items)
    if (page.items.length < pageSize || out.length >= page.total) break
    offset += pageSize
  }
  return out
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
export async function getTrades(pairAddr: string, limit = 50, before?: number): Promise<IndexerTrade[]> {
  const params = new URLSearchParams({ limit: limit.toString() })
  if (before) params.set('before', before.toString())
  return fetchJson<IndexerTrade[]>(`/api/v1/pairs/${pairAddr}/trades?${params}`)
}

/** Get 24h stats for a pair. */
export async function getPairStats(pairAddr: string): Promise<IndexerPairStats> {
  return fetchJson<IndexerPairStats>(`/api/v1/pairs/${pairAddr}/stats`)
}

export interface GetPairSubresourceParams {
  limit?: number
  before?: number
}

/** Add/remove liquidity history for a pair. */
export async function getPairLiquidityEvents(
  pairAddr: string,
  params?: GetPairSubresourceParams
): Promise<IndexerLiquidityEvent[]> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.before != null) sp.set('before', String(params.before))
  const qs = sp.toString()
  return fetchJson<IndexerLiquidityEvent[]>(`/api/v1/pairs/${pairAddr}/liquidity-events${qs ? `?${qs}` : ''}`)
}

/** Per-maker limit fills for a pair. */
export async function getPairLimitFills(
  pairAddr: string,
  params?: GetPairSubresourceParams
): Promise<IndexerLimitFill[]> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.before != null) sp.set('before', String(params.before))
  const qs = sp.toString()
  return fetchJson<IndexerLimitFill[]>(`/api/v1/pairs/${pairAddr}/limit-fills${qs ? `?${qs}` : ''}`)
}

/** Fills for a single on-chain order id. */
export async function getPairOrderLimitFills(
  pairAddr: string,
  orderId: number,
  limit = 50
): Promise<IndexerLimitFill[]> {
  const sp = new URLSearchParams({ limit: String(limit) })
  return fetchJson<IndexerLimitFill[]>(`/api/v1/pairs/${pairAddr}/limit-orders/${orderId}/fills?${sp}`)
}

/** Indexed `place_limit_order` events for a pair. */
export async function getPairLimitPlacements(
  pairAddr: string,
  params?: GetPairSubresourceParams
): Promise<IndexerLimitPlacement[]> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.before != null) sp.set('before', String(params.before))
  const qs = sp.toString()
  return fetchJson<IndexerLimitPlacement[]>(`/api/v1/pairs/${pairAddr}/limit-placements${qs ? `?${qs}` : ''}`)
}

/** Indexed `cancel_limit_order` events for a pair. */
export async function getPairLimitCancellations(
  pairAddr: string,
  params?: GetPairSubresourceParams
): Promise<IndexerLimitCancellation[]> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.before != null) sp.set('before', String(params.before))
  const qs = sp.toString()
  return fetchJson<IndexerLimitCancellation[]>(`/api/v1/pairs/${pairAddr}/limit-cancellations${qs ? `?${qs}` : ''}`)
}

/** On-chain book head for `side` (`bid` | `ask`) via indexer LCD proxy. */
export async function getPairOrderBookHead(
  pairAddr: string,
  side: 'bid' | 'ask'
): Promise<IndexerOrderBookHeadResponse> {
  const sp = new URLSearchParams({ side })
  return fetchJson<IndexerOrderBookHeadResponse>(`/api/v1/pairs/${pairAddr}/order-book-head?${sp}`)
}

/** Shallow on-chain book walk from head (depth default 10, max 20). */
export async function getPairLimitBookShallow(
  pairAddr: string,
  side: 'bid' | 'ask',
  depth = 10
): Promise<IndexerLimitBookShallowResponse> {
  const sp = new URLSearchParams({ side, depth: String(depth) })
  return fetchJson<IndexerLimitBookShallowResponse>(`/api/v1/pairs/${pairAddr}/limit-book-shallow?${sp}`)
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
export async function getTraderTrades(address: string, limit = 50, before?: number): Promise<IndexerTrade[]> {
  const params = new URLSearchParams({ limit: limit.toString() })
  if (before) params.set('before', before.toString())
  return fetchJson<IndexerTrade[]>(`/api/v1/traders/${address}/trades?${params}`)
}

/** Get trader leaderboard. */
export async function getLeaderboard(sort = 'total_volume', limit = 50): Promise<IndexerTrader[]> {
  const params = new URLSearchParams({ sort, limit: limit.toString() })
  return fetchJson<IndexerTrader[]>(`/api/v1/traders/leaderboard?${params}`)
}

/** Get trader's open positions with P&L. */
export async function getTraderPositions(address: string): Promise<IndexerPosition[]> {
  return fetchJson<IndexerPosition[]>(`/api/v1/traders/${address}/positions`)
}

/** All indexed tokens (metadata + ids for aggregators). */
export async function getTokens(): Promise<IndexerToken[]> {
  return fetchJson<IndexerToken[]>('/api/v1/tokens')
}

/** Token detail with per-window volume stats. */
export async function getTokenDetail(addrOrDenom: string): Promise<IndexerTokenDetail> {
  const enc = encodeURIComponent(addrOrDenom)
  return fetchJson<IndexerTokenDetail>(`/api/v1/tokens/${enc}`)
}

/** Pairs that include this token. */
export async function getTokenPairs(addrOrDenom: string): Promise<IndexerPair[]> {
  const enc = encodeURIComponent(addrOrDenom)
  return fetchJson<IndexerPair[]>(`/api/v1/tokens/${enc}/pairs`)
}

export interface GetHookEventsParams {
  hook_address?: string
  limit?: number
}

/** Recent hook execution events (burn, tax, etc.). */
export async function getHookEvents(params?: GetHookEventsParams): Promise<IndexerHookEvent[]> {
  const sp = new URLSearchParams()
  if (params?.hook_address?.trim()) sp.set('hook_address', params.hook_address.trim())
  if (params?.limit != null) sp.set('limit', String(params.limit))
  const qs = sp.toString()
  return fetchJson<IndexerHookEvent[]>(`/api/v1/hooks${qs ? `?${qs}` : ''}`)
}

/** Latest USTC/USD oracle snapshot (indexer-polled sources). */
export async function getOraclePrice(): Promise<IndexerOraclePriceResponse> {
  return fetchJson<IndexerOraclePriceResponse>('/api/v1/oracle/price')
}

export interface GetOracleHistoryParams {
  from?: string
  to?: string
  limit?: number
}

/** USTC/USD price history (defaults to last 24h if `from` omitted). */
export async function getOracleHistory(params?: GetOracleHistoryParams): Promise<IndexerOracleHistoryResponse> {
  const sp = new URLSearchParams()
  if (params?.from) sp.set('from', params.from)
  if (params?.to) sp.set('to', params.to)
  if (params?.limit != null) sp.set('limit', String(params.limit))
  const qs = sp.toString()
  return fetchJson<IndexerOracleHistoryResponse>(`/api/v1/oracle/history${qs ? `?${qs}` : ''}`)
}

export interface GetRouteSolveOptions {
  /** Per-hop hybrid split optimization via LCD (requires `amountIn`; indexer uses max 3 hops). */
  hybridOptimize?: boolean
  maxMakerFills?: number
}

/**
 * Multihop route from indexer graph (BFS, max 4 hops by default; **max 3 hops** when `hybridOptimize`).
 * **Limitation:** `token_in` / `token_out` must match indexed CW20 `contract_address` entries; native-only assets without a CW20 row are not routable via this endpoint.
 */
export async function getRouteSolve(
  tokenIn: string,
  tokenOut: string,
  amountIn?: string,
  options?: GetRouteSolveOptions
): Promise<IndexerRouteSolveResponse> {
  const sp = new URLSearchParams({ token_in: tokenIn.trim(), token_out: tokenOut.trim() })
  if (amountIn?.trim()) sp.set('amount_in', amountIn.trim())
  if (options?.hybridOptimize) sp.set('hybrid_optimize', 'true')
  if (options?.maxMakerFills != null) sp.set('max_maker_fills', String(options.maxMakerFills))
  return fetchJson<IndexerRouteSolveResponse>(`/api/v1/route/solve?${sp}`)
}

/** `POST /api/v1/route/solve` — merges `hybrid_by_hop` into router ops and optionally returns `estimated_amount_out` from LCD simulation. */
export async function postRouteSolve(
  tokenIn: string,
  tokenOut: string,
  amountIn: string | undefined,
  hybridByHop: (IndexerHybridHopInput | null)[]
): Promise<IndexerRouteSolveResponse> {
  return fetchJsonPost<IndexerRouteSolveResponse>('/api/v1/route/solve', {
    token_in: tokenIn.trim(),
    token_out: tokenOut.trim(),
    amount_in: amountIn?.trim() || null,
    hybrid_by_hop: hybridByHop,
  })
}
