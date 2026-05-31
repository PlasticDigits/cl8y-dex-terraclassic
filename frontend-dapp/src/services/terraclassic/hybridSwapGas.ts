import {
  EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP,
  HYBRID_SWAP_BASE_GAS,
  HYBRID_SWAP_GAS_FLOOR,
  HYBRID_SWAP_MAKER_GAS_BUFFER,
  HYBRID_SWAP_PER_EXPIRED_PARK_GAS,
  HYBRID_SWAP_PER_MAKER_GAS,
  HYBRID_SWAP_PER_SCAN_STEP_GAS,
  SWAP_GAS_BUFFER,
  SWAP_GAS_PER_HOP,
  SWAP_GAS_SAFETY_MARGIN,
  SWAP_MULTIHOP_GAS_PADDING_PER_HOP,
} from '@/utils/constants'
import type { HybridSwapParams } from '@/types'
import { MAX_EXPIRED_PARKS_PER_SWAP, MAX_SCAN_STEPS } from './hybridBookWalkLimits'

function gasLimitForPoolOnlySingleHop(): number {
  const hopCount = 1
  const scaled = Math.round(SWAP_GAS_PER_HOP * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor) + SWAP_GAS_SAFETY_MARGIN
}

/** Conservative ceiling when quote / `max_maker_fills` is unknown (GitLab #249 fallback). */
export const HYBRID_SWAP_GAS_LIMIT = 1_200_000

export type HybridSwapGasInput = {
  /** Distinct makers expected on the book leg (from quote or `max_maker_fills` cap). */
  makersUsed: number
  /** True when `pool_input > 0` on this hop. */
  hasPoolLeg: boolean
  /**
   * Optional indexer/LCD hint for head pollution; defaults to {@link MAX_SCAN_STEPS} offline (GitLab #260).
   */
  estimatedScanSteps?: number
  /** Optional hint; defaults to {@link MAX_EXPIRED_PARKS_PER_SWAP} offline. */
  estimatedExpiredParks?: number
}

/**
 * Extra gas for doubly-linked book walk beyond the maker-fill envelope.
 * Covers expired-prefix skips and parks up to on-chain caps without live book queries.
 */
export function bookWalkScanOverheadGas(
  makerUnits: number,
  options?: { scanSteps?: number; expiredParks?: number }
): number {
  const scanSteps = Math.min(MAX_SCAN_STEPS, Math.max(0, Math.floor(options?.scanSteps ?? MAX_SCAN_STEPS)))
  const extraScanSteps = Math.max(0, scanSteps - makerUnits)
  const expiredParks = Math.min(
    MAX_EXPIRED_PARKS_PER_SWAP,
    Math.max(0, Math.floor(options?.expiredParks ?? MAX_EXPIRED_PARKS_PER_SWAP))
  )
  return HYBRID_SWAP_PER_SCAN_STEP_GAS * extraScanSteps + HYBRID_SWAP_PER_EXPIRED_PARK_GAS * expiredParks
}

/**
 * Estimate gas for one direct pair `swap` with Pattern C hybrid.
 * Calibrated for post-#248 transfer aggregation; shallow books (0–2 makers) stay below {@link HYBRID_SWAP_GAS_LIMIT}.
 * Book legs add scan-step + expired-park overhead capped at on-chain limits (GitLab #260 / #254).
 */
export function gasLimitForHybridSwap(input: HybridSwapGasInput): number {
  const makers = Math.max(0, Math.floor(input.makersUsed))
  if (makers === 0) {
    return gasLimitForPoolOnlySingleHop()
  }
  const makerUnits = makers + HYBRID_SWAP_MAKER_GAS_BUFFER
  const makerComponent = HYBRID_SWAP_BASE_GAS + HYBRID_SWAP_PER_MAKER_GAS * makerUnits
  const scanOverhead = bookWalkScanOverheadGas(makerUnits, {
    scanSteps: input.estimatedScanSteps,
    expiredParks: input.estimatedExpiredParks,
  })
  const scaled = makerComponent + scanOverhead
  return Math.min(HYBRID_SWAP_GAS_LIMIT, Math.max(HYBRID_SWAP_GAS_FLOOR, scaled))
}

/** Read hybrid params from execute payloads for gas estimation. */
export function hybridSwapParamsFromRecord(hybrid: Record<string, unknown> | undefined): HybridSwapParams | undefined {
  if (!hybrid || typeof hybrid !== 'object') return undefined
  const pool = String(hybrid.pool_input ?? '0')
  const book = String(hybrid.book_input ?? '0')
  if (pool === '0' && book === '0') return undefined
  const maxFills = Number(hybrid.max_maker_fills ?? 0)
  return {
    pool_input: pool,
    book_input: book,
    max_maker_fills: Number.isFinite(maxFills) && maxFills > 0 ? Math.floor(maxFills) : 1,
    book_start_hint:
      hybrid.book_start_hint === null || hybrid.book_start_hint === undefined ? null : Number(hybrid.book_start_hint),
  }
}

/**
 * Makers attributed for gas from hybrid params on the wire.
 * Uses `max_maker_fills` as the quote cap; returns `undefined` when hybrid is missing (flat fallback).
 */
export function makersUsedForHybridGas(hybrid: HybridSwapParams | undefined): number | undefined {
  if (!hybrid) return undefined
  try {
    if (BigInt(hybrid.book_input) === 0n) return 0
  } catch {
    return undefined
  }
  if (hybrid.max_maker_fills < 1) return undefined
  return hybrid.max_maker_fills
}

export function gasLimitForHybridParams(hybrid: HybridSwapParams | undefined): number {
  const makers = makersUsedForHybridGas(hybrid)
  if (makers === undefined) return HYBRID_SWAP_GAS_LIMIT
  let hasPoolLeg = true
  try {
    hasPoolLeg = BigInt(hybrid!.pool_input) > 0n
  } catch {
    hasPoolLeg = true
  }
  return gasLimitForHybridSwap({ makersUsed: makers, hasPoolLeg })
}

/** Align on-chain cap with quote + buffer without exceeding the UI/indexer cap (GitLab #249). */
export function maxMakerFillsForSubmit(capFromQuote: number, makersUsedEstimate?: number): number {
  const cap = Math.max(1, Math.floor(capFromQuote))
  if (makersUsedEstimate == null || !Number.isFinite(makersUsedEstimate)) return cap
  const estimate = Math.max(0, Math.floor(makersUsedEstimate))
  return Math.min(cap, Math.max(1, estimate + HYBRID_SWAP_MAKER_GAS_BUFFER))
}

export function hybridParamsWithSubmitCap(hybrid: HybridSwapParams, makersUsedEstimate?: number): HybridSwapParams {
  return {
    ...hybrid,
    max_maker_fills: maxMakerFillsForSubmit(hybrid.max_maker_fills, makersUsedEstimate),
  }
}
