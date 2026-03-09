import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import type { DiscountResponse, RegistrationResponse, Tier, TierEntry } from '@/types'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'

export async function getTraderDiscount(
  traderAddr: string,
  senderAddr?: string
): Promise<DiscountResponse> {
  return queryContract<DiscountResponse>(FEE_DISCOUNT_CONTRACT_ADDRESS, {
    get_discount: {
      trader: traderAddr,
      sender: senderAddr ?? traderAddr,
    },
  })
}

export async function getTiers(): Promise<TierEntry[]> {
  const resp = await queryContract<{ tiers: TierEntry[] }>(FEE_DISCOUNT_CONTRACT_ADDRESS, {
    get_tiers: {},
  })
  return resp.tiers
}

export async function getTier(tierId: number): Promise<{ tier_id: number; tier: Tier }> {
  return queryContract<{ tier_id: number; tier: Tier }>(FEE_DISCOUNT_CONTRACT_ADDRESS, {
    get_tier: { tier_id: tierId },
  })
}

export async function getRegistration(traderAddr: string): Promise<RegistrationResponse> {
  return queryContract<RegistrationResponse>(FEE_DISCOUNT_CONTRACT_ADDRESS, {
    get_registration: { trader: traderAddr },
  })
}

export async function register(walletAddress: string, tierId: number): Promise<string> {
  return executeTerraContract(walletAddress, FEE_DISCOUNT_CONTRACT_ADDRESS, {
    register: { tier_id: tierId },
  })
}

export async function deregister(walletAddress: string): Promise<string> {
  return executeTerraContract(walletAddress, FEE_DISCOUNT_CONTRACT_ADDRESS, {
    deregister: {},
  })
}
