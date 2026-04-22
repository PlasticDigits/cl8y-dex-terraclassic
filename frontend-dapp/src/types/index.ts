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

/** TerraSwap-compatible simulation response */
export interface SimulationResponse {
  return_amount: string
  spread_amount: string
  commission_amount: string
}

/** TerraSwap-compatible reverse simulation response */
export interface ReverseSimulationResponse {
  offer_amount: string
  spread_amount: string
  commission_amount: string
}

/** Pattern C hybrid swap params (pair CW20 hook / router terra_swap). Amounts are raw integer strings. */
export interface HybridSwapParams {
  pool_input: string
  book_input: string
  max_maker_fills: number
  book_start_hint?: number | null
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
  /** 24h quote-side volume from indexed swaps (string integer) */
  volume_quote_24h?: string
}

/** Paginated response from `GET /api/v1/pairs` */
export interface IndexerPairsListResponse {
  items: IndexerPair[]
  total: number
  limit: number
  offset: number
}

export type IndexerPairSort = 'id' | 'fee' | 'created' | 'symbol' | 'volume_24h'

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
  open: string
  high: string
  low: string
  close: string
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
  price: string
  /** Hybrid / Pattern C when indexer has on-chain attrs */
  pool_return_amount?: string
  book_return_amount?: string
  limit_book_offer_consumed?: string
  effective_fee_bps?: number
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

/** `GET /api/v1/pairs/{addr}/limit-placements` */
export interface IndexerLimitPlacement {
  id: number
  pair_address: string
  block_height: number
  block_timestamp: string
  tx_hash: string
  order_id: number
  owner?: string | null
  side?: string | null
  price?: string | null
  expires_at?: number | null
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

/** Pair `is_paused` CosmWasm query */
export interface PairPausedResponse {
  paused: boolean
}

export interface IndexerPairStats {
  volume_base: string
  volume_quote: string
  trade_count: number
  high: string | null
  low: string | null
  open_price: string | null
  close_price: string | null
  price_change_pct: number | null
}

export interface IndexerOverview {
  total_volume_24h: string
  /** Quote-side 24h volume valued in USD (indexer oracle) */
  total_volume_24h_usd?: string
  total_trades_24h: number
  pair_count: number
  token_count: number
  /** Cached USTC/USD reference from indexer oracle; null if unavailable */
  ustc_price_usd?: string | null
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

/** `GET /api/v1/oracle/price` */
export interface IndexerOracleSourcePrice {
  source: string
  price_usd: string
  fetched_at: string
}

export interface IndexerOraclePriceResponse {
  price_usd: string | null
  sources: IndexerOracleSourcePrice[]
}

/** `GET /api/v1/oracle/history` */
export interface IndexerOracleHistoryEntry {
  price_usd: string
  fetched_at: string
}

export interface IndexerOracleHistoryResponse {
  prices: IndexerOracleHistoryEntry[]
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

export interface IndexerRouteSolveResponse {
  token_in: string
  token_out: string
  hops: IndexerRouteHop[]
  router_operations: unknown[]
  estimated_amount_out?: string
}

export interface IndexerTrader {
  address: string
  total_trades: number
  total_volume: string
  volume_24h: string
  volume_7d: string
  volume_30d: string
  tier_id: number | null
  tier_name: string | null
  registered: boolean
  first_trade_at: string | null
  last_trade_at: string | null
  total_realized_pnl: string
  best_trade_pnl: string
  worst_trade_pnl: string
  total_fees_paid: string
}

export interface IndexerPosition {
  pair_address: string
  asset_0_symbol: string
  asset_1_symbol: string
  net_position_quote: string
  avg_entry_price: string
  total_cost_base: string
  realized_pnl: string
  trade_count: number
}

export function isNativeDenom(tokenId: string): boolean {
  return tokenId === 'uluna' || tokenId === 'uusd'
}

export function getWrappedEquivalent(tokenId: string): string | null {
  return NATIVE_WRAPPED_PAIRS[tokenId] || null
}

export function getNativeEquivalent(tokenId: string): string | null {
  return WRAPPED_NATIVE_PAIRS[tokenId] || null
}
