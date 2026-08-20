import { NATIVE_WRAPPED_PAIRS, WRAPPED_NATIVE_PAIRS } from '@/utils/constants'

/** TerraSwap-compatible asset identifier */
export type AssetInfo = { token: { contract_addr: string } } | { native_token: { denom: string } }

/** TerraSwap-compatible asset */
export interface Asset {
  info: AssetInfo
  amount: string
}

/** TerraSwap-compatible pair info returned by queries */
export interface PairInfo {
  asset_infos: [AssetInfo, AssetInfo]
  contract_addr: string
  liquidity_token: string
}

export interface FeeConfig {
  fee_bps: number
  treasury: string
}

/** TerraSwap-compatible pool response */
export interface PoolResponse {
  assets: [Asset, Asset]
  total_share: string
}

/** Pattern C hybrid swap params (pair CW20 hook / router terra_swap). Amounts are raw integer strings. */
export interface HybridSwapParams {
  pool_input: string
  book_input: string
  max_maker_fills: number
  book_start_hint?: number | null
}

/** Pair `hybrid_simulation` / `hybrid_reverse_simulation` (only quote paths; GitLab #190). */
export interface HybridSimulationResponse {
  return_amount: string
  spread_amount: string
  commission_amount: string
  book_return_amount: string
  pool_return_amount: string
}

/** `hybrid_reverse_simulation` — `return_amount` is the required offer (`offer_amount` on chain). */
export interface HybridReverseSimulationResponse {
  offer_amount: string
  spread_amount: string
  commission_amount: string
  book_return_amount: string
  pool_return_amount: string
}

/** Helper: extract contract_addr from a CW20 AssetInfo, or return denom for native */
export function assetInfoLabel(info: AssetInfo): string {
  if ('token' in info) return info.token.contract_addr
  return info.native_token.denom
}

/** Helper: build an AssetInfo from a CW20 address or native denom */
export function tokenAssetInfo(identifier: string): AssetInfo {
  if (identifier.startsWith('terra1')) {
    return { token: { contract_addr: identifier } }
  }
  return { native_token: { denom: identifier } }
}

/** Fee discount tier definition */
export interface Tier {
  min_cl8y_balance: string
  discount_bps: number
  /** Placement-only discount (#514). Omitted → use `discount_bps`. */
  limit_discount_bps?: number | null
  governance_only: boolean
}

/** Fee discount tier entry with ID */
export interface TierEntry {
  tier_id: number
  tier: Tier
}

/** Fee discount query response */
export interface DiscountResponse {
  discount_bps: number
  /** Resolved limit-order placement discount (#514). Omitted on pre-#514 registries. */
  limit_discount_bps?: number | null
  needs_deregister: boolean
  registration_epoch: number | null
}

/** Fee discount registration response */
export interface RegistrationResponse {
  registered: boolean
  tier_id: number | null
  tier: Tier | null
}

/** TWAP Oracle observe response */
export interface ObserveResponse {
  price_a_cumulatives: string[]
  price_b_cumulatives: string[]
}

/** TWAP Oracle info response */
export interface OracleInfoResponse {
  observation_cardinality: number
  observation_index: number
  observations_stored: number
  oldest_observation_timestamp: number
  newest_observation_timestamp: number
}

/** Factory config response */
export interface FactoryConfigResponse {
  governance: string
  treasury: string
  default_fee_bps: number
  pair_code_id: number
  lp_token_code_id: number
  /** Uluna attached on `create_pair` when > 0 (GitLab #276, #345). */
  pair_creation_fee_uluna: string
  /** Canonical registry copied into new pairs at `create_pair` (GitLab #536). */
  discount_registry?: string | null
}

/** Hooks response from pair contract */
export interface HooksResponse {
  hooks: string[]
}

/** Tiers response from fee-discount contract */
export interface TiersResponse {
  tiers: TierEntry[]
}

/** Indexer API — asset row (CW20 and/or native) */
export interface IndexerAssetBrief {
  symbol: string
  contract_addr: string | null
  denom: string | null
  decimals: number
}

/** Indexer API types */
export interface IndexerPair {
  pair_address: string
  asset_0: IndexerAssetBrief
  asset_1: IndexerAssetBrief
  lp_token: string | null
  fee_bps: number | null
  is_active: boolean
  /** 24h quote-side volume from indexed swaps (raw integer; UI scales by `asset_1.decimals` — GitLab #534) */
  volume_quote_24h?: string
}

/** Paginated response from `GET /api/v1/pairs` */
export interface IndexerPairsListResponse {
  items: IndexerPair[]
  total: number
  limit: number
  offset: number
}

export type IndexerPairSort = 'id' | 'fee' | 'created' | 'symbol' | 'volume_24h' | 'relevance'

/** Map indexer pair metadata to on-chain `PairInfo` for pool queries and txs */
export function indexerAssetToAssetInfo(a: IndexerAssetBrief): AssetInfo {
  if (a.contract_addr) {
    return { token: { contract_addr: a.contract_addr } }
  }
  if (a.denom) {
    return { native_token: { denom: a.denom } }
  }
  throw new Error('Indexer asset has neither contract_addr nor denom')
}

export function indexerPairToPairInfo(p: IndexerPair): PairInfo {
  return {
    asset_infos: [indexerAssetToAssetInfo(p.asset_0), indexerAssetToAssetInfo(p.asset_1)],
    contract_addr: p.pair_address,
    liquidity_token: p.lp_token ?? '',
  }
}

export interface IndexerCandle {
  open_time: string
  /** Factory USD of 1 human `asset_0` (never human quote-per-base). GitLab #522 / #543. */
  open: string
  high: string
  low: string
  close: string
  /** Human quote-per-base OHLC for per-bar `invertUsd`. GitLab #543. */
  open_human?: string | null
  high_human?: string | null
  low_human?: string | null
  close_human?: string | null
  volume_base: string
  volume_quote: string
  trade_count: number
}

export interface IndexerTrade {
  id: number
  pair_address: string
  block_height: number
  block_timestamp: string
  tx_hash: string
  sender: string
  offer_asset: string
  ask_asset: string
  offer_amount: string
  return_amount: string
  /** Offer-asset decimals from indexed `assets` (GitLab #557). */
  offer_decimals?: number | null
  /** Ask-asset decimals from indexed `assets` (GitLab #557). */
  ask_decimals?: number | null
  price: string
  /** USD of 1 human unit of pair base (`asset_0`). GitLab #522. */
  price_usd?: string | null
  /** Hybrid / Pattern C when indexer has on-chain attrs */
  pool_return_amount?: string
  book_return_amount?: string
  limit_book_offer_consumed?: string
  effective_fee_bps?: number
  /** Indexed swap commission (raw units) when present — GitLab #163. */
  commission_amount?: string
  spread_amount?: string
}

/** `GET /api/v1/pairs/{addr}/limit-fills` */
export interface IndexerLimitFill {
  id: number
  pair_address: string
  swap_event_id: number | null
  block_height: number
  block_timestamp: string
  tx_hash: string
  order_id: number
  side: string
  maker: string
  price: string
  token0_amount: string
  token1_amount: string
  /** Pair `asset_0` decimals from indexed `assets` (GitLab #557). */
  token0_decimals?: number | null
  /** Pair `asset_1` decimals from indexed `assets` (GitLab #557). */
  token1_decimals?: number | null
  commission_amount: string
}

/** `GET /api/v1/pairs/{addr}/liquidity-events` */
export interface IndexerLiquidityEvent {
  id: number
  pair_address: string
  block_height: number
  block_timestamp: string
  tx_hash: string
  provider: string
  event_type: string
  asset_0_amount: string
  asset_1_amount: string
  lp_amount: string
}

/** `GET /api/v1/pairs/{addr}/limit-placements` or `GET /api/v1/traders/{addr}/limit-placements` — placements without a matching indexed cancel; trader route filters by `owner`. See GitLab #135, #217. Lifecycle: #142 / #141. */
export interface IndexerLimitPlacement {
  id: number
  pair_address: string
  block_height: number
  block_timestamp: string
  tx_hash: string
  order_id: number
  /** Indexer default listing includes `active` + `parked_expired`; omitted on legacy indexer responses → treat as `active`. */
  lifecycle_status?: string | null
  owner?: string | null
  side?: string | null
  price?: string | null
  expires_at?: number | null
  /** Raw escrow remaining when parked (`limit_order_expired_parked`); preserved after refund for UX. */
  remaining_escrow?: string | null
  parked_block_height?: number | null
  parked_block_timestamp?: string | null
  parked_tx_hash?: string | null
  refunded_block_height?: number | null
  refunded_block_timestamp?: string | null
  refunded_tx_hash?: string | null
}

/** `GET /api/v1/pairs/{addr}/limit-cancellations` */
export interface IndexerLimitCancellation {
  id: number
  pair_address: string
  block_height: number
  block_timestamp: string
  tx_hash: string
  order_id: number
  owner?: string | null
}

/** `GET /api/v1/pairs/{addr}/order-book-head` (indexer → LCD proxy) */
export interface IndexerOrderBookHeadResponse {
  head_order_id: number | null
}

/** One resting level from `GET /api/v1/pairs/{addr}/limit-book-shallow` */
export interface IndexerShallowLimitOrder {
  order_id: number
  owner: string
  side: string
  price: string
  remaining: string
  expires_at?: number | null
}

/** `GET /api/v1/pairs/{addr}/limit-book-shallow` */
export interface IndexerLimitBookShallowResponse {
  side: string
  orders: IndexerShallowLimitOrder[]
}

/** `GET /api/v1/pairs/{addr}/limit-book` (paginated on-chain book via indexer → LCD) */
export interface IndexerLimitBookPageResponse {
  side: string
  orders: IndexerShallowLimitOrder[]
  has_more: boolean
  next_after_order_id: number | null
}

/** One row from `GET /api/v1/pairs/{addr}/limit-book/insert-hints` (GitLab #267). */
export interface IndexerLimitBookInsertHintItem {
  price: string
  predecessor_order_id: number | null
  resolved: boolean
  reason?: 'head' | 'pagination_gap' | string | null
}

/** `GET /api/v1/pairs/{addr}/limit-book/insert-hints` */
export interface IndexerLimitBookInsertHintsResponse {
  side: string
  hints: IndexerLimitBookInsertHintItem[]
  budget_exhausted: boolean
}

/** Pair `is_paused` CosmWasm query */
export interface PairPausedResponse {
  paused: boolean
}

/**
 * Pair `OrderStatus` query (GitLab #505 / #530).
 * `unknown` is a successful decode — never map LCD/transport failures to this bucket (L21).
 */
export type PairOrderStatusKind = 'active' | 'parked_refund' | 'unknown'

export interface PairOrderStatusResponse {
  order_id: number
  status: PairOrderStatusKind | string
  owner?: string | null
  side?: string | null
  price?: string | null
  remaining?: string | null
  expires_at?: number | null
}

export interface IndexerPairStats {
  /** Raw oriented 24h base volume (integrator JSON). Scale in the UI with asset_0.decimals. */
  volume_base: string
  /** Raw oriented 24h quote volume (integrator JSON). Scale in the UI with asset_1.decimals. */
  volume_quote: string
  /** Human USD 24h notional from `SUM(swap_events.volume_usd)` (P522-Q). JSON `null` when trades > 0 but unpriced. GitLab #565. */
  volume_usd?: string | null
  trade_count: number
  high: string | null
  low: string | null
  open_price: string | null
  close_price: string | null
  price_change_pct: number | null
  /** USD OHLC of 1 human base (GitLab #522). */
  high_usd?: string | null
  low_usd?: string | null
  open_price_usd?: string | null
  close_price_usd?: string | null
}

export interface IndexerOverview {
  /** Raw mixed-decimal `SUM(offer_amount)` for integrators — Charts must not display this (#548). */
  total_volume_24h: string
  /** Human USD 24h volume. JSON `null` when trades > 0 but unpriced. `"0"` only when idle. */
  total_volume_24h_usd?: string | null
  total_trades_24h: number
  pair_count: number
  token_count: number
  /** Cached USTC/USD reference from indexer oracle; null if unavailable */
  ustc_price_usd?: string | null
  /** Additive GitLab #550 fields — optional until indexer ships. */
  total_volume_7d_usd?: string
  total_volume_30d_usd?: string
  total_trades_7d?: number
  total_trades_30d?: number
  tokens_added_30d?: number
  pairs_added_30d?: number
  active_pairs_24h?: number
  unique_traders_24h?: number
  /** DEX hub USD (#556). Additive; null when unresolved. */
  custc_price_usd?: string | null
  ust1_price_usd?: string | null
  ustr_price_usd?: string | null
}

/** `GET /api/v1/tokens` */
export interface IndexerToken {
  id: number
  contract_address: string | null
  denom: string | null
  is_cw20: boolean
  name: string
  symbol: string
  decimals: number
  logo_url: string | null
  coingecko_id: string | null
  cmc_id: number | null
}

/** Per-window volume from `GET /api/v1/tokens/{addr}` */
export interface IndexerVolumeStat {
  window: string
  volume: string
  volume_usd: string
  trade_count: number
  unique_traders: number
}

export interface IndexerTokenDetail {
  token: IndexerToken
  volume_stats: IndexerVolumeStat[]
}

/** `GET /api/v1/hooks` */
export interface IndexerHookEvent {
  id: number
  tx_hash: string
  hook_address: string
  action: string
  amount: string | number | null
  token: string | null
  skipped: string | null
  warning: string | null
  block_height: number
  block_time: string
}

/** `GET /api/v1/oracle/price` and `GET /api/v1/oracle/history` (catalog) */
export interface IndexerOracleTickerCatalogResponse {
  metadata: string
  /** Supported ticker path segments (e.g. `ustc`, `lunc`, `vfdusd`). */
  tickers: string[]
}

export interface IndexerOracleSourcePrice {
  source: string
  price_usd: string
  fetched_at: string
}

/** `GET /api/v1/oracle/price/{ticker}` */
export interface IndexerOraclePriceResponse {
  /** Path ticker (`ustc`, `lunc`, or `vfdusd`). */
  ticker: string
  price_usd: string | null
  sources: IndexerOracleSourcePrice[]
}

/** `GET /api/v1/oracle/history/{ticker}` */
export interface IndexerOracleHistoryEntry {
  price_usd: string
  fetched_at: string
}

export interface IndexerOracleHistoryResponse {
  /** Path ticker (`ustc`, `lunc`, or `vfdusd`). */
  ticker: string
  prices: IndexerOracleHistoryEntry[]
}

export interface IndexerHubPriceEntry {
  ticker: string
  asset_id?: number | null
  price_usd: string | null
  source_pair?: string | null
  /** Configured hub wrap CW20. Frontend still runs `getExplorerAddressUrl`. */
  asset_address?: string | null
  tvl_usd?: string | null
  updated_at?: string | null
}

/** `GET /api/v1/hub-prices` — DEX marks, not CEX (GitLab #556 / #570). */
export interface IndexerHubPricesResponse {
  metadata: string
  tickers: string[]
  prices: IndexerHubPriceEntry[]
}

/** One hop for `POST /api/v1/route/solve` `hybrid_by_hop` (matches on-chain `HybridSwapParams`). */
export interface IndexerHybridHopInput {
  pool_input: string
  book_input: string
  max_maker_fills: number
  book_start_hint?: number | null
}

/** `GET /api/v1/route/solve` — hops use CW20 addresses from indexer assets only */
export interface IndexerRouteHop {
  pair: string
  offer_token: string
  ask_token: string
}

/** Mirrors indexer `RouteQuoteKind` (snake_case JSON). */
export type IndexerRouteQuoteKind =
  | 'indexer_route_only'
  | 'indexer_pool_lcd'
  | 'indexer_hybrid_lcd'
  | 'indexer_hybrid_lcd_degraded'
  | 'indexer_pool_db'
  | 'indexer_hybrid_db'
  | 'indexer_hybrid_db_degraded'

export interface IndexerRouteSolveResponse {
  token_in: string
  token_out: string
  hops: IndexerRouteHop[]
  /** Full path: token_in, then each hop's ask_token (ends at token_out). */
  intermediate_tokens?: string[]
  quote_kind?: IndexerRouteQuoteKind
  hybrid_notes?: string | null
  router_operations: unknown[]
  estimated_amount_out?: string
  /** Fair output at best-route token cross-rate (raw integer). GitLab #293. */
  spot_amount_out?: string
  /** Symmetric deviation vs spot cross-rate (percent string). GitLab #293. */
  slippage_percent?: string
  token_in_price_quote?: string
  token_out_price_quote?: string
  /** `global_v2` when DB mirror grid is enabled. */
  solver_version?: string
  fidelity_check?: 'passed' | 'drift' | 'skipped'
  db_hybrid_queries?: number
}

/** Advisory in-flight solve status from `GET /api/v1/route/solve/progress` (GitLab #485). Not a quote. */
export interface IndexerRouteSolveProgress {
  stage: string
  done: number
  total: number
  label: string
  cache_hit: boolean
  updated_at_ms: number
}

export interface IndexerTrader {
  address: string
  total_trades: number
  total_volume: string
  /** P522-Q USD lifetime volume. Null when trades exist but none are priced (#553). */
  total_volume_usd?: string | null
  volume_24h: string
  volume_7d: string
  volume_30d: string
  tier_id: number | null
  tier_name: string | null
  registered: boolean
  first_trade_at: string | null
  last_trade_at: string | null
  total_realized_pnl: string
  best_trade_pnl: string | null
  worst_trade_pnl: string | null
  total_fees_paid: string
}

export interface IndexerPosition {
  pair_address: string
  asset_0_symbol: string
  asset_1_symbol: string
  /** Factory `asset_0` decimals — scale cost basis / realized P&L (GitLab #551). */
  asset_0_decimals?: number
  /** Factory `asset_1` (quote) decimals — scale net position (GitLab #551). */
  asset_1_decimals?: number
  asset_0_denom?: string | null
  asset_1_denom?: string | null
  net_position_quote: string
  avg_entry_price: string
  total_cost_base: string
  realized_pnl: string
  trade_count: number
}

export type { LimitBookTicketDraft } from './limitBookTicketDraft'

export function isNativeDenom(tokenId: string): boolean {
  return tokenId === 'uluna' || tokenId === 'uusd'
}

export function getWrappedEquivalent(tokenId: string): string | null {
  return NATIVE_WRAPPED_PAIRS[tokenId] || null
}

export function getNativeEquivalent(tokenId: string): string | null {
  return WRAPPED_NATIVE_PAIRS[tokenId] || null
}
