import { queryContract } from './queries'
import type { FeeConfig, HooksResponse, FactoryConfigResponse, TiersResponse } from '@/types'
import { FACTORY_CONTRACT_ADDRESS, FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'

/** Get factory config (governance, treasury, default fee, code IDs). */
export async function getFactoryConfig(): Promise<FactoryConfigResponse> {
  return queryContract<FactoryConfigResponse>(FACTORY_CONTRACT_ADDRESS, { config: {} })
}

/** Get fee config for a specific pair. */
export async function getPairFeeConfig(pairAddress: string): Promise<FeeConfig> {
  const resp = await queryContract<{ fee_config: FeeConfig }>(pairAddress, { get_fee_config: {} })
  return resp.fee_config
}

/** Get hooks list for a specific pair. */
export async function getPairHooks(pairAddress: string): Promise<string[]> {
  const resp = await queryContract<HooksResponse>(pairAddress, { get_hooks: {} })
  return resp.hooks
}

/** Get all fee discount tiers from the fee-discount contract. */
export async function getFeeDiscountTiers(): Promise<TiersResponse> {
  if (!FEE_DISCOUNT_CONTRACT_ADDRESS) {
    return { tiers: [] }
  }
  return queryContract<TiersResponse>(FEE_DISCOUNT_CONTRACT_ADDRESS, { get_tiers: {} })
}
