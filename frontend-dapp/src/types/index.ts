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
