import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { FAUCET_CONTRACT_ADDRESS } from '@/utils/constants'

export type FaucetConfigResponse = {
  admin: string
  drip_amount: string
  cooldown_seconds: number
  paused: boolean
  allowed_tokens: string[]
}

export type FaucetCooldownResponse = {
  can_claim: boolean
  seconds_remaining: number
  last_claim_at: string | null
  paused: boolean
}

export async function getFaucetConfig(): Promise<FaucetConfigResponse> {
  return queryContract<FaucetConfigResponse>(FAUCET_CONTRACT_ADDRESS, { config: {} })
}

export async function getFaucetCooldown(address: string): Promise<FaucetCooldownResponse> {
  return queryContract<FaucetCooldownResponse>(FAUCET_CONTRACT_ADDRESS, {
    cooldown: { address },
  })
}

export async function drip(walletAddress: string, tokenAddress: string): Promise<string> {
  return executeTerraContract(walletAddress, FAUCET_CONTRACT_ADDRESS, {
    drip: { token: tokenAddress },
  })
}
