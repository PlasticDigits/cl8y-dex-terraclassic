import type { IndexerOverview, LiquidityQuality } from '@/types'
import { formatProtocolUsd } from './formatProtocolStats'
import { DEFAULT_NETWORK, FACTORY_CONTRACT_ADDRESS, NETWORKS } from './constants'

/** Poll before a normal liquidity observation expires, with a finite request budget. */
export function protocolOverviewPollInterval(overview: IndexerOverview | undefined, now = Date.now()): number {
  if (overview?.liquidity_quality == null) return 60_000
  if (!['fresh', 'partial', 'verified_empty'].includes(overview.liquidity_quality)) return 10_000
  const ttl = overview.liquidity_max_age_seconds
  if (ttl == null || !Number.isFinite(ttl) || ttl <= 0) return 10_000
  const observed = overview.liquidity_as_of ? Date.parse(overview.liquidity_as_of) : NaN
  const remaining = ttl * 1000 - (now - observed)
  if (!Number.isFinite(remaining) || remaining <= 0 || observed > now + 1000) return 10_000
  return Math.max(5_000, Math.min(30_000, ttl * 500, remaining / 2))
}

/** Reject hostile numeric inputs before they can become USD text or SVG geometry. */
export function validLiquidityUsd(raw: string | null | undefined): string | null {
  if (typeof raw !== 'string' || raw.length > 80 || !/^\d+(?:\.\d+)?$/.test(raw)) return null
  return Number.isFinite(Number(raw)) ? raw : null
}

function liquidityObservationTime(asOf: unknown): number {
  if (
    typeof asOf !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(asOf)
  )
    return NaN
  const day = Date.parse(asOf.slice(0, 10))
  if (!Number.isFinite(day) || new Date(day).toISOString().slice(0, 10) !== asOf.slice(0, 10)) return NaN
  return Date.parse(asOf)
}

/** An empty observation cannot contain a positive total, even in malformed API payloads. */
export function liquidityQualityForValue(
  quality: LiquidityQuality | undefined,
  raw: string | null
): LiquidityQuality | undefined {
  return quality === 'verified_empty' && raw != null && Number(raw) !== 0 ? 'unknown' : quality
}

/** Retain only a public valuation; a partial fallback never replaces a complete total. */
export interface LastKnownLiquidity {
  value: string
  quality: 'fresh' | 'verified_empty' | 'partial'
  asOf: string
}

export function liquiditySnapshotKey(network: string, factory: string, indexer: string): string {
  return `cl8y:protocol-liquidity:v1:${JSON.stringify([network, factory, indexer])}`
}

export const PROTOCOL_LIQUIDITY_SNAPSHOT_KEY = liquiditySnapshotKey(
  NETWORKS[DEFAULT_NETWORK]?.terra.chainId ?? String(DEFAULT_NETWORK),
  FACTORY_CONTRACT_ADDRESS,
  import.meta.env.VITE_INDEXER_URL || 'http://127.0.0.1:3001'
)

export function parseLiquiditySnapshot(serialized: string | null, now = Date.now()): LastKnownLiquidity | null {
  if (serialized == null || serialized.length > 512) return null
  try {
    const parsed = JSON.parse(serialized)
    if (parsed == null || typeof parsed !== 'object' || Array.isArray(parsed)) return null
    if (Object.keys(parsed).sort().join(',') !== 'asOf,quality,value') return null
    const value = validLiquidityUsd(parsed.value)
    const observed = liquidityObservationTime(parsed.asOf)
    if (value == null || !Number.isFinite(observed) || observed <= 0 || observed > now) return null
    if (parsed.quality !== 'fresh' && parsed.quality !== 'verified_empty' && parsed.quality !== 'partial') return null
    if (parsed.quality === 'verified_empty' && Number(value) !== 0) return null
    return { value, quality: parsed.quality, asOf: parsed.asOf }
  } catch {
    return null
  }
}

export function readLiquiditySnapshot(now = Date.now()): LastKnownLiquidity | null {
  try {
    return parseLiquiditySnapshot(localStorage.getItem(PROTOCOL_LIQUIDITY_SNAPSHOT_KEY), now)
  } catch {
    return null
  }
}

export function writeLiquiditySnapshot(snapshot: LastKnownLiquidity): void {
  try {
    localStorage.setItem(PROTOCOL_LIQUIDITY_SNAPSHOT_KEY, JSON.stringify(snapshot))
  } catch {
    // Storage denial/quota must not interrupt in-memory display retention.
  }
}

export function retainQualifiedLiquidity(
  previous: LastKnownLiquidity | null,
  overview: IndexerOverview | undefined,
  now = Date.now()
): LastKnownLiquidity | null {
  const current = overviewLiquidityDisplay(overview, now)
  if ((!current.comparable && current.quality !== 'partial') || !overview?.liquidity_as_of) return previous
  if (current.quality === 'partial' && previous != null && previous.quality !== 'partial') return previous
  const next = parseLiquiditySnapshot(
    JSON.stringify({ value: overview.total_liquidity_usd, quality: current.quality, asOf: overview.liquidity_as_of }),
    now
  )
  if (next == null) return previous
  if (previous?.asOf === next.asOf && previous.value === next.value && previous.quality === next.quality)
    return previous
  return next
}

export function overviewLiquidityDisplay(
  overview: IndexerOverview | undefined,
  now = Date.now(),
  retained: LastKnownLiquidity | null = null
) {
  const raw = validLiquidityUsd(overview?.total_liquidity_usd)
  let quality = overview?.liquidity_quality
  const asOf = overview?.liquidity_as_of
  const observed = liquidityObservationTime(asOf)
  const ttl = overview?.liquidity_max_age_seconds
  // Volume polling/cache updates must not relabel an older valuation as fresh.
  if (quality === 'fresh' || quality === 'verified_empty' || quality === 'partial') {
    if (!Number.isFinite(observed) || observed > now) quality = 'unavailable'
    else if (!Number.isFinite(ttl) || ttl == null || ttl <= 0 || now - observed > ttl * 1000) quality = 'stale'
  }
  // A legacy zero does not prove empty reserves, even when the priced count is zero.
  const legacyZero = quality == null && raw != null && Number(raw) === 0
  if (legacyZero || raw == null) quality = 'unavailable'
  quality = liquidityQualityForValue(quality, raw)
  const known = quality === 'fresh' || quality === 'verified_empty' || quality === 'partial' || quality == null
  let value = known && raw != null ? formatProtocolUsd(raw) : '—'
  const useRetained =
    retained != null &&
    !(retained.quality === 'partial' && (quality === 'fresh' || quality === 'verified_empty')) &&
    (quality !== retained.quality || retained.asOf !== asOf || retained.value !== raw)
  if (useRetained) {
    value = formatProtocolUsd(retained.value)
  }
  return {
    quality: useRetained ? retained.quality : quality,
    value,
    // Backend only supplies a delta for matching, qualified baseline coverage.
    comparable: !useRetained && (quality === 'fresh' || quality === 'verified_empty'),
  }
}
