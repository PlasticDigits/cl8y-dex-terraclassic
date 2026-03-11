/** TerraSwap-compatible asset identifier */
export type AssetInfo =
  | { token: { contract_addr: string } }
  | { native_token: { denom: string } }

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

/** Helper: extract contract_addr from a CW20 AssetInfo, or return denom for native */
export function assetInfoLabel(info: AssetInfo): string {
  if ('token' in info) return info.token.contract_addr
  return info.native_token.denom
}

/** Helper: build a CW20 AssetInfo */
export function tokenAssetInfo(contractAddr: string): AssetInfo {
  return { token: { contract_addr: contractAddr } }
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
}

/** Fee discount registration response */
export interface RegistrationResponse {
  registered: boolean
  tier_id: number | null
  tier: Tier | null
}

/** TWAP Oracle observe response */
export interface ObserveResponse {
  tick_cumulatives: string[]
}

/** TWAP Oracle info response */
export interface OracleInfoResponse {
  observation_cardinality: number
  observation_index: number
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

/** Indexer API types */
export interface IndexerPair {
  contract_address: string
  asset_0: { symbol: string; contract_addr: string; decimals: number }
  asset_1: { symbol: string; contract_addr: string; decimals: number }
  lp_token: string | null
  fee_bps: number | null
  is_active: boolean
}

export interface IndexerCandle {
  open_time: string
  open_price: string
  high_price: string
  low_price: string
  close_price: string
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
  total_trades_24h: number
  pair_count: number
  token_count: number
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
