import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import {
  WRAP_MAPPER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  WRAPPED_NATIVE_PAIRS,
} from '@/utils/constants'

interface DenomMappingResponse {
  denom: string
  cw20_addr: string
}

interface RateLimitResponse {
  config: { max_amount_per_window: string; window_seconds: number } | null
  current_window_start: string | null
  amount_used: string
}

export async function wrapViaTreasury(walletAddress: string, denom: string, amount: string): Promise<string> {
  return executeTerraContract(walletAddress, TREASURY_CONTRACT_ADDRESS, { wrap_deposit: {} }, [{ denom, amount }])
}

export async function unwrap(
  walletAddress: string,
  cw20Address: string,
  amount: string,
  recipient?: string
): Promise<string> {
  const unwrapMsg = btoa(JSON.stringify({ unwrap: { recipient: recipient ?? null } }))
  return executeTerraContract(walletAddress, cw20Address, {
    send: {
      contract: WRAP_MAPPER_CONTRACT_ADDRESS,
      amount,
      msg: unwrapMsg,
    },
  })
}

export async function queryDenomMapping(denom: string): Promise<DenomMappingResponse> {
  return queryContract<DenomMappingResponse>(WRAP_MAPPER_CONTRACT_ADDRESS, {
    denom_mapping: { denom },
  })
}

export async function queryRateLimit(denom: string): Promise<RateLimitResponse> {
  return queryContract<RateLimitResponse>(WRAP_MAPPER_CONTRACT_ADDRESS, {
    rate_limit: { denom },
  })
}

interface ConfigResponse {
  governance: string
  treasury: string
  paused: boolean
  fee_bps: number
}

export async function queryPausedState(): Promise<boolean> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return false
  try {
    const config = await queryContract<ConfigResponse>(WRAP_MAPPER_CONTRACT_ADDRESS, { config: {} })
    return config.paused
  } catch {
    return false
  }
}

export async function checkRateLimitExceeded(denom: string, wrapAmount: string): Promise<boolean> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return false
  try {
    const rl = await queryRateLimit(denom)
    if (!rl.config) return false
    const maxAmount = BigInt(rl.config.max_amount_per_window)
    const used = BigInt(rl.amount_used)
    return used + BigInt(wrapAmount) > maxAmount
  } catch {
    return false
  }
}

export function isNativeWrappedPair(tokenA: string, tokenB: string): boolean {
  return NATIVE_WRAPPED_PAIRS[tokenA] === tokenB || NATIVE_WRAPPED_PAIRS[tokenB] === tokenA
}

export function getWrappedForNative(denom: string): string | null {
  return NATIVE_WRAPPED_PAIRS[denom] || null
}

export function getNativeForWrapped(cw20Addr: string): string | null {
  return WRAPPED_NATIVE_PAIRS[cw20Addr] || null
}

export function isNativeToken(tokenId: string): boolean {
  return tokenId === 'uluna' || tokenId === 'uusd'
}

export function isWrappedNative(tokenId: string): boolean {
  return tokenId in WRAPPED_NATIVE_PAIRS
}
