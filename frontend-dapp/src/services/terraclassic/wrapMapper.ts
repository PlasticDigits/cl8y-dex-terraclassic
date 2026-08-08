import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import {
  WRAP_MAPPER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  WRAPPED_NATIVE_PAIRS,
} from '@/utils/constants'
import { bpsToPercentLabel } from '@/utils/limitOrderFeeSummary'

interface DenomMappingResponse {
  denom: string
  cw20_addr: string
}

interface RateLimitResponse {
  config: { max_amount_per_window: string; window_seconds: number } | null
  current_window_start: string | null
  amount_used: string
}

export interface WrapMapperConfigResponse {
  governance: string
  treasury: string
  paused: boolean
  fee_bps: number
}

const CONFIG_CACHE_MS = 30_000
let cachedConfig: { at: number; value: WrapMapperConfigResponse } | null = null

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

/**
 * Query wrap-mapper `Config` (paused + fee_bps). Cached ~30s.
 * GitLab #507 — UI/sim must use on-chain fee_bps (mainnet expect 100).
 */
export async function queryWrapMapperConfig(): Promise<WrapMapperConfigResponse | null> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return null
  const now = Date.now()
  if (cachedConfig && now - cachedConfig.at < CONFIG_CACHE_MS) return cachedConfig.value
  try {
    const config = await queryContract<WrapMapperConfigResponse>(WRAP_MAPPER_CONTRACT_ADDRESS, { config: {} })
    cachedConfig = { at: now, value: config }
    return config
  } catch {
    return null
  }
}

/** Test helper — clear in-memory config cache. */
export function clearWrapMapperConfigCache(): void {
  cachedConfig = null
}

export async function queryWrapMapperFeeBps(): Promise<number> {
  const config = await queryWrapMapperConfig()
  if (!config) return 0
  const bps = Number(config.fee_bps)
  return Number.isFinite(bps) && bps > 0 ? Math.floor(bps) : 0
}

export async function queryPausedState(): Promise<boolean> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return false
  try {
    const config = await queryWrapMapperConfig()
    return config?.paused === true
  } catch {
    return false
  }
}

/**
 * Net amount after wrap-mapper fee skim.
 * Matches router `net_after_wrap_mapper_unwrap_fee`: `amount - floor(amount × fee_bps / 10_000)`.
 * Applies on both wrap mint and unwrap redeem (GitLab #507).
 */
export function netAfterWrapMapperFee(amount: bigint, feeBps: number): bigint {
  if (amount <= 0n) return 0n
  const bps = Math.floor(Number(feeBps))
  if (!Number.isFinite(bps) || bps <= 0) return amount
  const clamped = BigInt(Math.min(bps, 10_000))
  const fee = (amount * clamped) / 10_000n
  return amount - fee
}

/**
 * Minimum pre-fee amount such that `netAfterWrapMapperFee(amount, feeBps) >= targetNet`.
 * Used when inverting pool provide auto-fill from pool-ratio net → user gross (#507).
 */
export function amountForTargetNetAfterWrapMapperFee(targetNet: bigint, feeBps: number): bigint {
  if (targetNet <= 0n) return 0n
  const bps = Math.floor(Number(feeBps))
  if (!Number.isFinite(bps) || bps <= 0) return targetNet
  if (bps >= 10_000) return targetNet
  const den = 10_000n - BigInt(bps)
  let amount = (targetNet * 10_000n + den - 1n) / den
  while (netAfterWrapMapperFee(amount, bps) < targetNet) {
    amount += 1n
  }
  return amount
}

/** Direct wrap/unwrap route note — never claim 1:1 when fee_bps > 0. */
export function wrapUnwrapFeeNote(kind: 'wrap' | 'unwrap', feeBps: number): string {
  const bps = Math.floor(Number(feeBps))
  const label = kind === 'wrap' ? 'Wrap' : 'Unwrap'
  if (!Number.isFinite(bps) || bps <= 0) return `${label} (1:1)`
  return `${label} (${bpsToPercentLabel(bps)} fee)`
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
