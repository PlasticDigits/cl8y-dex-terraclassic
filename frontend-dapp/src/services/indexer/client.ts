import { parseIndexerTraderPayload } from '@/services/indexer/traderProfilePayload'
import { normalizeLimitBookPageResponse } from '@/utils/limitBookPagination'
import { parseProtocolOracleTicker } from '@/utils/protocolOracleTicker'
import { parseHubPriceTicker } from '@/utils/hubPriceTicker'
import {
  isAllowlistedProtocolVolumeLimit,
  isProtocolVolumeGrain,
  type ProtocolVolumeGrain,
} from '@/utils/protocolVolumeGrain'
import type {
  IndexerPair,
  IndexerPairsListResponse,
  IndexerPairSort,
  IndexerCandle,
  IndexerTrade,
  IndexerPairStats,
  IndexerOverview,
  ProtocolFeesResponse,
  ProtocolVolumeDailyResponse,
  ProtocolVolumeSeriesResponse,
  IndexerTrader,
  IndexerPosition,
  IndexerToken,
  IndexerTokenDetail,
  IndexerHookEvent,
  IndexerOraclePriceResponse,
  IndexerOracleTickerCatalogResponse,
  IndexerOracleHistoryResponse,
  IndexerOracleVenusVfdusd,
  IndexerHubPricesResponse,
  IndexerHubPriceEntry,
  IndexerHybridHopInput,
  IndexerRouteSolveResponse,
  IndexerRouteSolveProgress,
  IndexerLimitFill,
  IndexerLiquidityEvent,
  IndexerLimitPlacement,
  IndexerLimitCancellation,
  IndexerOrderBookHeadResponse,
  IndexerLimitBookInsertHintsResponse,
  IndexerLimitBookShallowResponse,
  IndexerLimitBookPageResponse,
} from '@/types'

export const INDEXER_URL = import.meta.env.VITE_INDEXER_URL || 'http://127.0.0.1:3001'

/** Encode a single URL path segment (bech32 addresses, denoms, order ids). */
function pathSegment(value: string | number): string {
  return encodeURIComponent(String(value).trim())
}

const FETCH_TIMEOUT_MS = import.meta.env.VITE_E2E_INDEXER_OUTAGE === '1' ? 4_000 : 15_000
/**
 * Distant CW20 pairs (e.g. JADE→RUBY) can take 20–30s+ on production hybrid `global_v1`
 * solves. The default 15s timeout aborts mid-solve and forces a slow client BFS fallback
 * while React Query still shows Calculating (GitLab #484).
 */
const ROUTE_SOLVE_TIMEOUT_MS = import.meta.env.VITE_E2E_INDEXER_OUTAGE === '1' ? 4_000 : 45_000
const MAX_RETRIES = import.meta.env.VITE_E2E_INDEXER_OUTAGE === '1' ? 0 : 1

/**
 * Max `limit` for trader history CSV exports — matches indexer
 * `GET /api/v1/traders/{addr}/…` clamp (`limit` ≤ 200). Do not request higher;
 * the server silently clamps and the UI must not imply a larger export.
 * GitLab #479.
 */
export const TRADER_HISTORY_CSV_MAX_LIMIT = 200

/** Exported for unit tests — default indexer HTTP timeout (ms). */
export const INDEXER_FETCH_TIMEOUT_MS = FETCH_TIMEOUT_MS
/** Exported for unit tests — GET/POST `/route/solve` timeout (ms). */
export const INDEXER_ROUTE_SOLVE_TIMEOUT_MS = ROUTE_SOLVE_TIMEOUT_MS

type IndexerFetchInit = RequestInit & {
  /** Override per-request timeout (defaults to {@link FETCH_TIMEOUT_MS}). */
  timeoutMs?: number
}

function isRetryableFetchError(err: Error): boolean {
  return err.name === 'AbortError' || err.message.includes('Failed to fetch') || err.message.includes('NetworkError')
}

/**
 * Combine a caller AbortSignal (e.g. React Query) with a timeout AbortController.
 * Either abort source cancels the in-flight fetch (#484).
 */
function mergeTimeoutSignal(
  timeoutMs: number,
  external?: AbortSignal | null
): { signal: AbortSignal; cleanup: () => void } {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  const onExternalAbort = () => controller.abort()
  if (external) {
    if (external.aborted) {
      controller.abort()
    } else {
      external.addEventListener('abort', onExternalAbort)
    }
  }
  return {
    signal: controller.signal,
    cleanup: () => {
      clearTimeout(timer)
      external?.removeEventListener('abort', onExternalAbort)
    },
  }
}

/** Shared timed fetch with one network/timeout retry (parity for JSON + CSV). */
async function fetchText(path: string, init?: IndexerFetchInit): Promise<string> {
  const { timeoutMs = FETCH_TIMEOUT_MS, signal: externalSignal, ...rest } = init ?? {}
  let lastError: Error | undefined
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    // Do not retry when the caller (React Query) cancelled the quote — only timeout/network.
    if (externalSignal?.aborted) {
      throw externalSignal.reason instanceof Error
        ? externalSignal.reason
        : new DOMException('The operation was aborted.', 'AbortError')
    }
    const { signal, cleanup } = mergeTimeoutSignal(timeoutMs, externalSignal)
    try {
      const resp = await fetch(`${INDEXER_URL}${path}`, {
        ...rest,
        signal,
      })
      if (!resp.ok) {
        throw new Error(`Indexer API error: ${resp.status} ${resp.statusText}`)
      }
      return await resp.text()
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err))
      if (externalSignal?.aborted) throw lastError
      if (!isRetryableFetchError(lastError) || attempt >= MAX_RETRIES) throw lastError
    } finally {
      cleanup()
    }
  }
  throw lastError!
}

async function fetchJson<T>(path: string, init?: IndexerFetchInit): Promise<T> {
  const text = await fetchText(path, init)
  try {
    return JSON.parse(text) as T
  } catch {
    throw new Error(`Indexer returned invalid JSON for ${path}`)
  }
}

async function fetchJsonPost<T>(path: string, body: unknown, init?: IndexerFetchInit): Promise<T> {
  return fetchJson<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
    ...init,
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
  return fetchJson<IndexerPair>(`/api/v1/pairs/${pathSegment(pairAddr)}`)
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
  return fetchJson<IndexerCandle[]>(`/api/v1/pairs/${pathSegment(pairAddr)}/candles?${params}`)
}

/** Get recent trades for a pair. */
export async function getTrades(pairAddr: string, limit = 50, before?: number): Promise<IndexerTrade[]> {
  const params = new URLSearchParams({ limit: limit.toString() })
  if (before) params.set('before', before.toString())
  return fetchJson<IndexerTrade[]>(`/api/v1/pairs/${pathSegment(pairAddr)}/trades?${params}`)
}

/** Get 24h stats for a pair (`volume_usd` is human USD; `volume_base` / `volume_quote` stay raw). */
export async function getPairStats(pairAddr: string): Promise<IndexerPairStats> {
  return fetchJson<IndexerPairStats>(`/api/v1/pairs/${pathSegment(pairAddr)}/stats`)
}

export interface GetPairSubresourceParams {
  limit?: number
  before?: number
}

/** Query params for {@link getPairLimitPlacements} (`?status=` matches indexer — GitLab #142). */
export interface GetPairLimitPlacementsParams extends GetPairSubresourceParams {
  status?: string
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
  return fetchJson<IndexerLiquidityEvent[]>(
    `/api/v1/pairs/${pathSegment(pairAddr)}/liquidity-events${qs ? `?${qs}` : ''}`
  )
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
  return fetchJson<IndexerLimitFill[]>(`/api/v1/pairs/${pathSegment(pairAddr)}/limit-fills${qs ? `?${qs}` : ''}`)
}

/** Fills for a single on-chain order id. */
export async function getPairOrderLimitFills(
  pairAddr: string,
  orderId: number,
  limit = 50
): Promise<IndexerLimitFill[]> {
  const sp = new URLSearchParams({ limit: String(limit) })
  return fetchJson<IndexerLimitFill[]>(
    `/api/v1/pairs/${pathSegment(pairAddr)}/limit-orders/${pathSegment(orderId)}/fills?${sp}`
  )
}

/** Indexed `place_limit_order` events excluding rows with an indexed `cancel_limit_order` for the same pair + `order_id` (GitLab #135). Default **`active` + `parked_expired`** (GitLab #142). */
export async function getPairLimitPlacements(
  pairAddr: string,
  params?: GetPairLimitPlacementsParams
): Promise<IndexerLimitPlacement[]> {
  const sp = new URLSearchParams()
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.before != null) sp.set('before', String(params.before))
  if (params?.status != null && params.status.trim() !== '') sp.set('status', params.status.trim())
  const qs = sp.toString()
  return fetchJson<IndexerLimitPlacement[]>(
    `/api/v1/pairs/${pathSegment(pairAddr)}/limit-placements${qs ? `?${qs}` : ''}`
  )
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
  return fetchJson<IndexerLimitCancellation[]>(
    `/api/v1/pairs/${pathSegment(pairAddr)}/limit-cancellations${qs ? `?${qs}` : ''}`
  )
}

/** On-chain book head for `side` (`bid` | `ask`) via indexer LCD proxy. */
export async function getPairOrderBookHead(
  pairAddr: string,
  side: 'bid' | 'ask'
): Promise<IndexerOrderBookHeadResponse> {
  const sp = new URLSearchParams({ side })
  return fetchJson<IndexerOrderBookHeadResponse>(`/api/v1/pairs/${pathSegment(pairAddr)}/order-book-head?${sp}`)
}

/** Shallow on-chain book walk from head (depth default 10, max 20). */
export async function getPairLimitBookShallow(
  pairAddr: string,
  side: 'bid' | 'ask',
  depth = 10
): Promise<IndexerLimitBookShallowResponse> {
  const sp = new URLSearchParams({ side, depth: String(depth) })
  return fetchJson<IndexerLimitBookShallowResponse>(`/api/v1/pairs/${pathSegment(pairAddr)}/limit-book-shallow?${sp}`)
}

export interface GetPairLimitBookPageParams {
  limit?: number
  afterOrderId?: number
  /** Inclusive band bounds — both required for price-window fetch (GitLab #267). */
  priceFrom?: string
  priceTo?: string
}

/** Paginated on-chain book (default limit 50, max 100 per request). */
export async function getPairLimitBookPage(
  pairAddr: string,
  side: 'bid' | 'ask',
  params?: GetPairLimitBookPageParams
): Promise<IndexerLimitBookPageResponse> {
  const sp = new URLSearchParams({ side })
  if (params?.limit != null) sp.set('limit', String(params.limit))
  if (params?.afterOrderId != null) sp.set('after_order_id', String(params.afterOrderId))
  if (params?.priceFrom != null) sp.set('price_from', params.priceFrom)
  if (params?.priceTo != null) sp.set('price_to', params.priceTo)
  const raw = await fetchJson<IndexerLimitBookPageResponse>(`/api/v1/pairs/${pathSegment(pairAddr)}/limit-book?${sp}`)
  return normalizeLimitBookPageResponse(raw)
}

/** Batch insert-hint resolution for ladder rung prices (indexer only — GitLab #267). */
export async function getPairLimitBookInsertHints(
  pairAddr: string,
  side: 'bid' | 'ask',
  prices: string[]
): Promise<IndexerLimitBookInsertHintsResponse> {
  const sp = new URLSearchParams({ side, prices: prices.join(',') })
  return fetchJson<IndexerLimitBookInsertHintsResponse>(
    `/api/v1/pairs/${pathSegment(pairAddr)}/limit-book/insert-hints?${sp}`
  )
}

/** Get global DEX overview stats. */
export async function getOverview(): Promise<IndexerOverview> {
  return fetchJson<IndexerOverview>('/api/v1/overview')
}

const PROTOCOL_FEE_WINDOWS = new Set(['24h', '7d', '30d'])

/** Protocol treasury fee breakdown. `window` allowlisted — never pass `?ticker=`. */
export async function getProtocolFees(window: '24h' | '7d' | '30d' = '24h'): Promise<ProtocolFeesResponse> {
  if (!PROTOCOL_FEE_WINDOWS.has(window)) {
    throw new Error('Invalid protocol fee window')
  }
  return fetchJson<ProtocolFeesResponse>(`/api/v1/protocol/fees?window=${window}`)
}

const PROTOCOL_VOLUME_DAILY_DAYS = new Set([7, 30])

/** UTC-day Protocol volume series. `days` allowlisted — never Llama `from`/`to` / `?ticker=`. */
export async function getProtocolVolumeDaily(days: 7 | 30 = 7): Promise<ProtocolVolumeDailyResponse> {
  if (!PROTOCOL_VOLUME_DAILY_DAYS.has(days)) {
    throw new Error('Invalid protocol volume daily days')
  }
  return fetchJson<ProtocolVolumeDailyResponse>(`/api/v1/protocol/volume/daily?days=${days}`)
}

/** UTC grain series. `grain` + `limit` allowlisted before fetch (GitLab #668). */
export async function getProtocolVolumeSeries(
  grain: ProtocolVolumeGrain,
  limit: number
): Promise<ProtocolVolumeSeriesResponse> {
  if (!isProtocolVolumeGrain(grain) || !isAllowlistedProtocolVolumeLimit(grain, limit)) {
    throw new Error('Invalid protocol volume grain or limit')
  }
  return fetchJson<ProtocolVolumeSeriesResponse>(`/api/v1/protocol/volume/daily?grain=${grain}&limit=${limit}`)
}

/** Cached fee-discount registry LCD probe (GitLab #365). */
export async function getFeeDiscountHealth(): Promise<{
  configured: boolean
  fee_discount_registry_ok: boolean | null
  consecutive_lcd_failures: number
}> {
  return fetchJson('/api/v1/health/fee-discount')
}

/** Get trader profile. */
export async function getTrader(address: string): Promise<IndexerTrader> {
  const raw = await fetchJson<unknown>(`/api/v1/traders/${pathSegment(address)}`)
  return parseIndexerTraderPayload(raw)
}

/** Get trader's historical swaps (indexed `swap_events` where sender matches). Optional `pair` scopes to one pair. */
export async function getTraderTrades(
  address: string,
  opts?: { limit?: number; before?: number; pair?: string }
): Promise<IndexerTrade[]> {
  const limit = opts?.limit ?? 50
  const params = new URLSearchParams({ limit: limit.toString() })
  if (opts?.before != null) params.set('before', String(opts.before))
  if (opts?.pair?.trim()) params.set('pair', opts.pair.trim())
  return fetchJson<IndexerTrade[]>(`/api/v1/traders/${pathSegment(address)}/trades?${params}`)
}

/** Per-wallet limit fills (indexed maker) — optional `pair` scopes to one pair contract. */
export async function getTraderLimitFills(
  address: string,
  opts?: GetPairSubresourceParams & { pair?: string }
): Promise<IndexerLimitFill[]> {
  const sp = new URLSearchParams()
  if (opts?.limit != null) sp.set('limit', String(opts.limit))
  if (opts?.before != null) sp.set('before', String(opts.before))
  if (opts?.pair?.trim()) sp.set('pair', opts.pair.trim())
  const qs = sp.toString()
  return fetchJson<IndexerLimitFill[]>(`/api/v1/traders/${pathSegment(address)}/limit-fills${qs ? `?${qs}` : ''}`)
}

/** Per-wallet indexed limit cancellations (owner attribute) — optional `pair` filter. */
export async function getTraderLimitCancellations(
  address: string,
  opts?: GetPairSubresourceParams & { pair?: string }
): Promise<IndexerLimitCancellation[]> {
  const sp = new URLSearchParams()
  if (opts?.limit != null) sp.set('limit', String(opts.limit))
  if (opts?.before != null) sp.set('before', String(opts.before))
  if (opts?.pair?.trim()) sp.set('pair', opts.pair.trim())
  const qs = sp.toString()
  return fetchJson<IndexerLimitCancellation[]>(
    `/api/v1/traders/${pathSegment(address)}/limit-cancellations${qs ? `?${qs}` : ''}`
  )
}

/** Wallet-wide open limit placements (indexed owner; GitLab #217). */
export async function getTraderLimitPlacements(
  address: string,
  opts?: GetPairLimitPlacementsParams & { pair?: string }
): Promise<IndexerLimitPlacement[]> {
  const sp = new URLSearchParams()
  if (opts?.limit != null) sp.set('limit', String(opts.limit))
  if (opts?.before != null) sp.set('before', String(opts.before))
  if (opts?.status != null && opts.status.trim() !== '') sp.set('status', opts.status.trim())
  if (opts?.pair?.trim()) sp.set('pair', opts.pair.trim())
  const qs = sp.toString()
  return fetchJson<IndexerLimitPlacement[]>(
    `/api/v1/traders/${pathSegment(address)}/limit-placements${qs ? `?${qs}` : ''}`
  )
}

export type TraderHistoryCsvResource = 'trades' | 'limit-fills' | 'limit-cancellations'

/** Download CSV from trader history endpoints (`format=csv`). Retries once on network/timeout like `fetchJson`. */
export async function fetchTraderHistoryCsv(
  resource: TraderHistoryCsvResource,
  address: string,
  opts?: { limit?: number; pair?: string }
): Promise<string> {
  const limit = Math.min(opts?.limit ?? TRADER_HISTORY_CSV_MAX_LIMIT, TRADER_HISTORY_CSV_MAX_LIMIT)
  const sp = new URLSearchParams({ format: 'csv', limit: String(limit) })
  if (
    opts?.pair?.trim() &&
    (resource === 'trades' || resource === 'limit-fills' || resource === 'limit-cancellations')
  ) {
    sp.set('pair', opts.pair.trim())
  }
  const path =
    resource === 'trades'
      ? `/api/v1/traders/${pathSegment(address)}/trades?${sp}`
      : `/api/v1/traders/${pathSegment(address)}/${resource}?${sp}`
  return fetchText(path)
}

/** Trigger a browser download of CSV text (UTF-8). */
export function downloadTextAsFile(filename: string, text: string, mime = 'text/csv;charset=utf-8') {
  const blob = new Blob([text], { type: mime })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.rel = 'noopener'
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}

/** Get trader leaderboard. Charts + `/trader` Volume tab uses `total_volume_usd` (#553 / #657). */
export async function getLeaderboard(sort = 'total_volume_usd', limit = 50): Promise<IndexerTrader[]> {
  const params = new URLSearchParams({ sort, limit: limit.toString() })
  return fetchJson<IndexerTrader[]>(`/api/v1/traders/leaderboard?${params}`)
}

/** Get trader's open positions with P&L. */
export async function getTraderPositions(address: string): Promise<IndexerPosition[]> {
  return fetchJson<IndexerPosition[]>(`/api/v1/traders/${pathSegment(address)}/positions`)
}

/** All indexed tokens (metadata + ids for aggregators). */
export async function getTokens(): Promise<IndexerToken[]> {
  return fetchJson<IndexerToken[]>('/api/v1/tokens')
}

/** Token detail with per-window volume stats. */
export async function getTokenDetail(addrOrDenom: string): Promise<IndexerTokenDetail> {
  const enc = pathSegment(addrOrDenom)
  return fetchJson<IndexerTokenDetail>(`/api/v1/tokens/${enc}`)
}

export async function getCommunityTokens(params?: {
  manager?: string
  includeUnattested?: boolean
}): Promise<import('@/types').IndexerCommunityTokenList> {
  const sp = new URLSearchParams()
  if (params?.manager) sp.set('manager', params.manager)
  if (params?.includeUnattested) sp.set('include_unattested', '1')
  const qs = sp.toString()
  return fetchJson(`/api/v1/community-tokens${qs ? `?${qs}` : ''}`)
}

/** Pairs that include this token. */
export async function getTokenPairs(addrOrDenom: string): Promise<IndexerPair[]> {
  const enc = pathSegment(addrOrDenom)
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

/** Available USTC/LUNC/vFDUSD oracle tickers and usage metadata (GitLab #515 / #550). */
export async function getOraclePriceCatalog(): Promise<IndexerOracleTickerCatalogResponse> {
  return fetchJson<IndexerOracleTickerCatalogResponse>('/api/v1/oracle/price')
}

/**
 * Latest external USD reference for an allowlisted ticker (indexer-polled CEX/aggregator sources).
 * Unknown / injected ticker values fall back to `ustc` and are never interpolated raw (GitLab #550).
 * @param ticker `ustc` | `lunc` | `vfdusd`; defaults to `ustc`.
 */
export async function getOraclePrice(ticker = 'ustc'): Promise<IndexerOraclePriceResponse> {
  const safe = parseProtocolOracleTicker(ticker)
  return fetchJson<IndexerOraclePriceResponse>(`/api/v1/oracle/price/${pathSegment(safe)}`)
}

/**
 * Venus Core Pool redeem snapshot (FDUSD per 1 vFDUSD). Hardcoded `vfdusd` path —
 * never interpolates `?ticker=` (GitLab #571). Browser must not call BSC.
 */
export async function getOracleVenusVfdusd(): Promise<IndexerOracleVenusVfdusd> {
  return fetchJson<IndexerOracleVenusVfdusd>(`/api/v1/oracle/price/${pathSegment('vfdusd')}/venus`)
}

export interface GetOracleHistoryParams {
  /** `ustc` | `lunc` | `vfdusd`; defaults to `ustc`. Unknown values → `ustc`. */
  ticker?: string
  from?: string
  to?: string
  limit?: number
}

/**
 * External USD reference history for an allowlisted ticker (defaults to last 24h if `from` omitted).
 * @param params.ticker `ustc`, `lunc`, or `vfdusd`; defaults to `ustc`.
 */
export async function getOracleHistory(params?: GetOracleHistoryParams): Promise<IndexerOracleHistoryResponse> {
  const ticker = parseProtocolOracleTicker(params?.ticker ?? 'ustc')
  const sp = new URLSearchParams()
  if (params?.from) sp.set('from', params.from)
  if (params?.to) sp.set('to', params.to)
  if (params?.limit != null) sp.set('limit', String(params.limit))
  const qs = sp.toString()
  return fetchJson<IndexerOracleHistoryResponse>(`/api/v1/oracle/history/${pathSegment(ticker)}${qs ? `?${qs}` : ''}`)
}

/** DEX hub USD snapshot (`custc` / `lunc` / `ust1` / `ustr`). Not CEX; do not call `getOraclePrice('ustr')`. */
export async function getHubPrices(): Promise<IndexerHubPricesResponse> {
  return fetchJson<IndexerHubPricesResponse>('/api/v1/hub-prices')
}

/**
 * One DEX hub mark. Unknown / injected ticker is ignored (no fetch).
 * @param ticker `custc` | `lunc` | `ust1` | `ustr`
 */
export async function getHubPrice(ticker: string): Promise<IndexerHubPriceEntry | null> {
  const safe = parseHubPriceTicker(ticker)
  if (!safe) return null
  return fetchJson<IndexerHubPriceEntry>(`/api/v1/hub-prices/${pathSegment(safe)}`)
}

export interface GetRouteSolveOptions {
  /** Legacy pool-only routing (`pool_only=true`; max 4 hops, `hybrid: null`). */
  poolOnly?: boolean
  /** @deprecated Hybrid optimization is default when `amountIn` is set; use `poolOnly` to opt out. */
  hybridOptimize?: boolean
  maxMakerFills?: number
  /** Connected wallet for CL8Y fee-tier discounted quotes (GitLab #245). */
  trader?: string
  sender?: string
  /** React Query (or other) cancellation — aborts the in-flight solve (GitLab #484). */
  signal?: AbortSignal
}

/**
 * Multihop route from indexer graph. GET defaults to **hybrid-aware** routing (max **3 hops**) when
 * `amountIn` is set. Pass `poolOnly: true` for legacy pool-only ops (max 4 hops).
 * Uses a longer HTTP timeout than other indexer reads ({@link INDEXER_ROUTE_SOLVE_TIMEOUT_MS})
 * because distant-pair hybrid solves often exceed 15s (GitLab #484).
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
  if (options?.poolOnly) sp.set('pool_only', 'true')
  else if (options?.hybridOptimize === false) sp.set('hybrid_optimize', 'false')
  if (options?.maxMakerFills != null) sp.set('max_maker_fills', String(options.maxMakerFills))
  if (options?.trader?.trim()) sp.set('trader', options.trader.trim())
  if (options?.sender?.trim()) sp.set('sender', options.sender.trim())
  return fetchJson<IndexerRouteSolveResponse>(`/api/v1/route/solve?${sp}`, {
    signal: options?.signal,
    timeoutMs: ROUTE_SOLVE_TIMEOUT_MS,
  })
}

/**
 * Advisory progress for an in-flight hybrid solve (GitLab #485).
 * Same query params as {@link getRouteSolve}; uses the default indexer timeout (not 45s).
 * Progress is not a quote — never treat `label` as HTML.
 */
export async function getRouteSolveProgress(
  tokenIn: string,
  tokenOut: string,
  amountIn: string,
  options?: Pick<GetRouteSolveOptions, 'maxMakerFills' | 'trader' | 'sender' | 'signal'>
): Promise<IndexerRouteSolveProgress> {
  const sp = new URLSearchParams({
    token_in: tokenIn.trim(),
    token_out: tokenOut.trim(),
    amount_in: amountIn.trim(),
  })
  if (options?.maxMakerFills != null) sp.set('max_maker_fills', String(options.maxMakerFills))
  if (options?.trader?.trim()) sp.set('trader', options.trader.trim())
  if (options?.sender?.trim()) sp.set('sender', options.sender.trim())
  return fetchJson<IndexerRouteSolveProgress>(`/api/v1/route/solve/progress?${sp}`, {
    signal: options?.signal,
  })
}

/** `POST /api/v1/route/solve` — merges `hybrid_by_hop` into router ops and optionally returns `estimated_amount_out` from LCD simulation. */
export async function postRouteSolve(
  tokenIn: string,
  tokenOut: string,
  amountIn: string | undefined,
  hybridByHop: (IndexerHybridHopInput | null)[],
  options?: Pick<GetRouteSolveOptions, 'trader' | 'sender' | 'signal'>
): Promise<IndexerRouteSolveResponse> {
  return fetchJsonPost<IndexerRouteSolveResponse>(
    '/api/v1/route/solve',
    {
      token_in: tokenIn.trim(),
      token_out: tokenOut.trim(),
      amount_in: amountIn?.trim() || null,
      hybrid_by_hop: hybridByHop,
      trader: options?.trader?.trim() || null,
      sender: options?.sender?.trim() || null,
    },
    {
      signal: options?.signal,
      timeoutMs: ROUTE_SOLVE_TIMEOUT_MS,
    }
  )
}
