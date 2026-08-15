import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import {
  WRAP_MAPPER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  WRAPPED_NATIVE_PAIRS,
} from '@/utils/constants'
import { bpsToPercentLabel } from '@/utils/limitOrderFeeSummary'
import { formatBurnTaxPercentLabel } from '@/utils/nativeTransferTax'
import { deriveWrapRateLimitStatus, type WrapRateLimitResponse } from '@/utils/wrapRateLimit'

interface DenomMappingResponse {
  denom: string
  cw20_addr: string
}

export type RateLimitResponse = WrapRateLimitResponse

export type WrapMapperFeeKind = 'wrap' | 'unwrap'

/** Normalized wrap-mapper `Config` after parse (GitLab #516). */
export interface WrapMapperConfigResponse {
  governance: string
  treasury: string
  paused: boolean
  fee_wrap_bps: number
  fee_unwrap_bps: number
}

/**
 * LCD / fixture shape. Post ustr-cmm#9 migrate, `Config` drops `fee_bps` (no dual-read).
 * Pre-migrate columbus-5 still returns `{ fee_bps }` only — parser maps that to both sides.
 */
export interface RawWrapMapperConfig {
  governance?: string
  treasury?: string
  paused?: boolean
  fee_bps?: number | string | null
  fee_wrap_bps?: number | string | null
  fee_unwrap_bps?: number | string | null
}

/** In-memory LCD cache bound — stale quotes after gov `set_fees` last at most this long (W14). */
export const WRAP_MAPPER_CONFIG_CACHE_MS = 30_000

let cachedConfig: { at: number; value: WrapMapperConfigResponse } | null = null

function parseNonNegativeBps(value: unknown): number | null {
  if (value == null || value === '') return null
  const n = Number(value)
  if (!Number.isFinite(n) || n < 0) return null
  return Math.floor(n)
}

function hasFeeField(value: unknown): boolean {
  return value != null && value !== ''
}

/**
 * Fail closed on partial/invalid split fees. Transitional `{ fee_bps }` only when both
 * split fields are absent (pre-migrate). Never treat missing fields as 0% (W13).
 */
export function parseWrapMapperFeePair(
  raw: RawWrapMapperConfig | null | undefined
): { fee_wrap_bps: number; fee_unwrap_bps: number } | null {
  if (!raw) return null
  const hasWrap = hasFeeField(raw.fee_wrap_bps)
  const hasUnwrap = hasFeeField(raw.fee_unwrap_bps)
  if (hasWrap || hasUnwrap) {
    const wrap = parseNonNegativeBps(raw.fee_wrap_bps)
    const unwrap = parseNonNegativeBps(raw.fee_unwrap_bps)
    if (wrap == null || unwrap == null) return null
    return { fee_wrap_bps: wrap, fee_unwrap_bps: unwrap }
  }
  const legacy = parseNonNegativeBps(raw.fee_bps)
  if (legacy == null) return null
  return { fee_wrap_bps: legacy, fee_unwrap_bps: legacy }
}

export function parseWrapMapperConfig(raw: RawWrapMapperConfig | null | undefined): WrapMapperConfigResponse | null {
  const fees = parseWrapMapperFeePair(raw)
  if (!raw || !fees) return null
  return {
    governance: String(raw.governance ?? ''),
    treasury: String(raw.treasury ?? ''),
    paused: raw.paused === true,
    ...fees,
  }
}

export function wrapMapperFeeBps(
  config: RawWrapMapperConfig | WrapMapperConfigResponse | null | undefined,
  kind: WrapMapperFeeKind
): number | null {
  const fees = parseWrapMapperFeePair(config)
  if (!fees) return null
  return kind === 'wrap' ? fees.fee_wrap_bps : fees.fee_unwrap_bps
}

/**
 * `fee_unwrap_bps` so user unwrap all-in ≈ 2% (`receive/A = 0.98`) after InstantWithdraw tax.
 * Prefer ≤2% when rounding. Escalate if tax ≥ ~2% — cannot hit 2% without subsidy/gross-up.
 * Docs/ops only — UI quotes always use on-chain config (W3 / W14).
 */
export function retuneUnwrapFeeBps(burnTaxRate: number): number {
  if (!Number.isFinite(burnTaxRate) || burnTaxRate < 0 || burnTaxRate >= 0.02) {
    throw new Error('Cannot hit ≈2% unwrap all-in when burn tax ≥ ~2% without subsidy')
  }
  return Math.round(10_000 - 9800 / (1 - burnTaxRate))
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

/**
 * Query wrap-mapper `Config` (paused + split fees + treasury). Cached ~30s (W14).
 * GitLab #516 — UI/sim must use on-chain `fee_wrap_bps` / `fee_unwrap_bps`.
 * Returns null when LCD fails or fees are missing/partial — fail closed (never 0%).
 */
export async function queryWrapMapperConfig(): Promise<WrapMapperConfigResponse | null> {
  if (!WRAP_MAPPER_CONTRACT_ADDRESS) return null
  const now = Date.now()
  if (cachedConfig && now - cachedConfig.at < WRAP_MAPPER_CONFIG_CACHE_MS) return cachedConfig.value
  try {
    const raw = await queryContract<RawWrapMapperConfig>(WRAP_MAPPER_CONTRACT_ADDRESS, { config: {} })
    const config = parseWrapMapperConfig(raw)
    if (!config) return null
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
 * On-chain wrap-mapper fee for wrap mint or unwrap redeem. Throws when config is
 * unavailable so simulate/execute never silently assume fee-free 1:1 (#507 M1 / #516 W13).
 */
export async function queryWrapMapperFeeBps(kind: WrapMapperFeeKind): Promise<number> {
  const config = await queryWrapMapperConfig()
  const bps = wrapMapperFeeBps(config, kind)
  if (bps == null) throw new Error('Wrap mapper config unavailable')
  return bps
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
export function wrapTreasuryMatchesEnv(
  config: Pick<WrapMapperConfigResponse, 'treasury'> | RawWrapMapperConfig
): boolean {
  const envTreasury = TREASURY_CONTRACT_ADDRESS.trim()
  const onChain = (config.treasury ?? '').trim()
  return envTreasury.length > 0 && onChain.length > 0 && envTreasury === onChain
}

/**
 * Net amount after wrap-mapper fee skim.
 * Matches router `net_after_wrap_mapper_unwrap_fee`: `amount - floor(amount × fee_bps / 10_000)`.
 * Callers pass `fee_wrap_bps` for mint and `fee_unwrap_bps` for redeem (#507 / #516).
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
 * Direct wrap/unwrap route note — never claim 1:1 when fee is unknown or bps > 0.
 * Pass the matching wrap or unwrap bps (W12). Pass `null`/`undefined` when config failed.
 *
 * Unwrap notes optionally include the chain burn-tax rate on InstantWithdraw (#512).
 * Keep this to a single fee line (W7) — no permanent educational paragraphs.
 */
export function wrapUnwrapFeeNote(
  kind: 'wrap' | 'unwrap',
  feeBps: number | null | undefined,
  burnTaxRate?: string | null
): string {
  const label = kind === 'wrap' ? 'Wrap' : 'Unwrap'
  if (feeBps == null || !Number.isFinite(Number(feeBps))) return `${label} fee unavailable`
  const bps = Math.floor(Number(feeBps))
  if (kind === 'unwrap') {
    const taxLabel = burnTaxRate != null ? formatBurnTaxPercentLabel(burnTaxRate) : null
    if (bps <= 0) {
      return taxLabel ? `${label} (1:1 mapper; ${taxLabel} burn tax on payout)` : `${label} (1:1)`
    }
    return taxLabel
      ? `${label} (${bpsToPercentLabel(bps)} fee; You Receive after ${taxLabel} burn tax)`
      : `${label} (${bpsToPercentLabel(bps)} fee; You Receive after burn tax)`
  }
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
