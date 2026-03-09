import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { FACTORY_CONTRACT_ADDRESS } from '@/utils/constants'
import type { PairInfo } from '@/types'

interface ConfigResponse {
  governance: string
  treasury: string
  default_fee_bps: number
  pair_code_id: number
  lp_token_code_id: number
}

interface PairsResponse {
  pairs: PairInfo[]
  next: string | null
}

interface CodeIdsResponse {
  code_ids: number[]
  next: number | null
}

export async function getFactoryConfig(): Promise<ConfigResponse> {
  return queryContract<ConfigResponse>(FACTORY_CONTRACT_ADDRESS, { get_config: {} })
}

export async function getAllPairs(startAfter?: string, limit?: number): Promise<PairsResponse> {
  return queryContract<PairsResponse>(FACTORY_CONTRACT_ADDRESS, {
    get_all_pairs: { start_after: startAfter, limit },
  })
}

export async function getPair(tokenA: string, tokenB: string): Promise<PairInfo> {
  const resp = await queryContract<{ pair: PairInfo }>(FACTORY_CONTRACT_ADDRESS, {
    get_pair: { token_a: tokenA, token_b: tokenB },
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
    create_pair: { token_a: tokenA, token_b: tokenB },
  })
}
