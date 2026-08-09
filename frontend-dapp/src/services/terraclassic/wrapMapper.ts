import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import {
  WRAP_MAPPER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  WRAPPED_NATIVE_PAIRS,
} from '@/utils/constants'
import { bpsToPercentLabel } from '@/utils/limitOrderFeeSummary'
import { deriveWrapRateLimitStatus, type WrapRateLimitResponse } from '@/utils/wrapRateLimit'

interface DenomMappingResponse {
  denom: string
  cw20_addr: string
}

export type RateLimitResponse = WrapRateLimitResponse

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
 * Query wrap-mapper `Config` (paused + fee_bps + treasury). Cached ~30s.
 * GitLab #507 — UI/sim must use on-chain fee_bps (mainnet expect 100).
 * Returns null when LCD fails — callers must fail closed (never treat as fee_bps=0).
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

/**
 * On-chain wrap-mapper fee_bps. Throws when config is unavailable so simulate/execute
 * never silently assume fee-free 1:1 (GitLab #507 review M1).
 */
export async function queryWrapMapperFeeBps(): Promise<number> {
  const config = await queryWrapMapperConfig()
  if (!config) throw new Error('Wrap mapper config unavailable')
  const bps = Number(config.fee_bps)
  if (!Number.isFinite(bps) || bps < 0) throw new Error('Invalid wrap mapper fee_bps')
  return Math.floor(bps)
}

/**
 * Pause gate. `null` = LCD/config unavailable → UI must fail closed (do not assume unpaused).
 */
export async function queryPausedState(): Promise<boolean | null> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return false
  const config = await queryWrapMapperConfig()
  if (!config) return null
  return config.paused === true
}

/**
 * True when Coolify `VITE_TREASURY_ADDRESS` matches on-chain mapper `config.treasury`.
 * Mismatch misroutes wrap_deposit (GitLab #507 review M3 / W2).
 */
export function wrapTreasuryMatchesEnv(config: WrapMapperConfigResponse): boolean {
  const envTreasury = TREASURY_CONTRACT_ADDRESS.trim()
  const onChain = (config.treasury ?? '').trim()
  return envTreasury.length > 0 && onChain.length > 0 && envTreasury === onChain
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
  if (bps >= 10_000) return 0n
  const den = 10_000n - BigInt(bps)
  let amount = (targetNet * 10_000n + den - 1n) / den
  while (netAfterWrapMapperFee(amount, bps) < targetNet) {
    amount += 1n
  }
  return amount
}

/**
 * Direct wrap/unwrap route note — never claim 1:1 when fee is unknown or fee_bps > 0.
 * Pass `null`/`undefined` when mapper config has not loaded successfully.
 */
export function wrapUnwrapFeeNote(kind: 'wrap' | 'unwrap', feeBps: number | null | undefined): string {
  const label = kind === 'wrap' ? 'Wrap' : 'Unwrap'
  if (feeBps == null || !Number.isFinite(Number(feeBps))) return `${label} fee unavailable`
  const bps = Math.floor(Number(feeBps))
  if (bps <= 0) return `${label} (1:1)`
  return `${label} (${bpsToPercentLabel(bps)} fee)`
}

/**
 * Rate-limit gate. `null` = LCD unavailable → UI must fail closed (do not assume unlimited).
 * Expired windows are treated as full capacity (chain resets on next wrap).
 */
export async function checkRateLimitExceeded(denom: string, wrapAmount: string): Promise<boolean | null> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return false
  try {
    const rl = await queryRateLimit(denom)
    const status = deriveWrapRateLimitStatus(rl, Math.floor(Date.now() / 1000))
    if (!status) return false
    return BigInt(wrapAmount) > status.remainingRaw
  } catch {
    return null
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
