/** TerraSwap-style asset id */
export type AssetInfo = { token: { contract_addr: string } } | { native_token: { denom: string } }

export interface Asset {
  info: AssetInfo
  amount: string
}

export interface PairInfo {
  asset_infos: [AssetInfo, AssetInfo]
  contract_addr: string
  liquidity_token: string
}

export interface PoolResponse {
  assets: [Asset, Asset]
  total_share: string
}

export interface HybridSwapParams {
  pool_input: string
  book_input: string
  max_maker_fills: number
  book_start_hint?: number | null
}

export function assetInfoLabel(info: AssetInfo): string {
  if ('token' in info) return info.token.contract_addr
  return info.native_token.denom
}

export function tokenAssetInfo(addrOrDenom: string): AssetInfo {
  if (addrOrDenom.startsWith('terra1')) {
    return { token: { contract_addr: addrOrDenom } }
  }
  return { native_token: { denom: addrOrDenom } }
}

export interface SwapOperation {
  terra_swap: {
    offer_asset_info: AssetInfo
    ask_asset_info: AssetInfo
    hybrid?: HybridSwapParams | null
  }
}
