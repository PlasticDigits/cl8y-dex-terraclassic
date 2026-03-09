import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { FACTORY_CONTRACT_ADDRESS } from '@/utils/constants'
import type { AssetInfo, PairInfo } from '@/types'
import { tokenAssetInfo } from '@/types'

interface ConfigResponse {
  governance: string
  treasury: string
  default_fee_bps: number
  pair_code_id: number
  lp_token_code_id: number
}

interface PairsResponse {
  pairs: PairInfo[]
}

interface PairResponse {
  pair: PairInfo
}

interface CodeIdsResponse {
  code_ids: number[]
  next: number | null
}

export async function getFactoryConfig(): Promise<ConfigResponse> {
  return queryContract<ConfigResponse>(FACTORY_CONTRACT_ADDRESS, { config: {} })
}

export async function getAllPairs(
  startAfter?: [AssetInfo, AssetInfo],
  limit?: number
): Promise<PairsResponse> {
  return queryContract<PairsResponse>(FACTORY_CONTRACT_ADDRESS, {
    pairs: { start_after: startAfter, limit },
  })
}

export async function getPair(assetInfos: [AssetInfo, AssetInfo]): Promise<PairInfo> {
  const resp = await queryContract<PairResponse>(FACTORY_CONTRACT_ADDRESS, {
    pair: { asset_infos: assetInfos },
  })
  return resp.pair
}

export async function getWhitelistedCodeIds(
  startAfter?: number,
  limit?: number
): Promise<CodeIdsResponse> {
  return queryContract<CodeIdsResponse>(FACTORY_CONTRACT_ADDRESS, {
    get_whitelisted_code_ids: { start_after: startAfter, limit },
  })
}

export async function createPair(
  walletAddress: string,
  tokenA: string,
  tokenB: string
): Promise<string> {
  return executeTerraContract(walletAddress, FACTORY_CONTRACT_ADDRESS, {
    create_pair: {
      asset_infos: [tokenAssetInfo(tokenA), tokenAssetInfo(tokenB)],
    },
  })
}
