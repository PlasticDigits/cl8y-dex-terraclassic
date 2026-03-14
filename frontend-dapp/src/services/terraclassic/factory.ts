import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { FACTORY_CONTRACT_ADDRESS } from '@/utils/constants'
import type { AssetInfo, PairInfo } from '@/types'
import { tokenAssetInfo } from '@/types'

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

export async function getAllPairs(startAfter?: [AssetInfo, AssetInfo], limit?: number): Promise<PairsResponse> {
  return queryContract<PairsResponse>(FACTORY_CONTRACT_ADDRESS, {
    pairs: { start_after: startAfter, limit },
  })
}

export async function getAllPairsPaginated(maxPairs = 200): Promise<PairsResponse> {
  const PAGE_SIZE = 50
  const allPairs: PairInfo[] = []
  let startAfter: [AssetInfo, AssetInfo] | undefined

  while (allPairs.length < maxPairs) {
    const resp = await getAllPairs(startAfter, PAGE_SIZE)
    if (resp.pairs.length === 0) break
    allPairs.push(...resp.pairs)
    if (resp.pairs.length < PAGE_SIZE) break
    const last = resp.pairs[resp.pairs.length - 1]
    startAfter = last.asset_infos
  }

  return { pairs: allPairs }
}

export async function getPair(assetInfos: [AssetInfo, AssetInfo]): Promise<PairInfo> {
  const resp = await queryContract<PairResponse>(FACTORY_CONTRACT_ADDRESS, {
    pair: { asset_infos: assetInfos },
  })
  return resp.pair
}

export async function getWhitelistedCodeIds(startAfter?: number, limit?: number): Promise<CodeIdsResponse> {
  return queryContract<CodeIdsResponse>(FACTORY_CONTRACT_ADDRESS, {
    get_whitelisted_code_ids: { start_after: startAfter, limit },
  })
}

export async function createPair(walletAddress: string, tokenA: string, tokenB: string): Promise<string> {
  return executeTerraContract(walletAddress, FACTORY_CONTRACT_ADDRESS, {
    create_pair: {
      asset_infos: [tokenAssetInfo(tokenA), tokenAssetInfo(tokenB)],
    },
  })
}
