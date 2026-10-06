import type { IndexerOverview, LiquidityQuality } from '@/types'
import { formatProtocolUsd } from './formatProtocolStats'
import { formatRelativeAge, RELATIVE_AGE_FALLBACK } from './formatDate'
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

export function liquidityQualityLabel(quality: LiquidityQuality | undefined): string {
  switch (quality) {
    case 'fresh':
      return 'Fresh'
    case 'verified_empty':
      return 'Verified empty'
    case 'partial':
      return 'Partial · lower bound'
    case 'stale':
      return 'Stale · current liquidity unknown'
    case 'unavailable':
      return 'Unavailable · liquidity unknown'
    default:
      return 'Coverage unknown'
  }
}

/** Only this public, complete valuation is retained; never an overview or wallet record. */
export interface LastKnownLiquidity {
  value: string
  quality: 'fresh' | 'verified_empty'
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
    if (parsed.quality !== 'fresh' && parsed.quality !== 'verified_empty') return null
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
  if (!current.comparable || !overview?.liquidity_as_of) return previous
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
  let value = known && raw != null ? `${quality === 'partial' ? '≥ ' : ''}${formatProtocolUsd(raw)}` : '—'
  let reason = (() => {
    switch (quality) {
      case 'fresh':
        return 'Value of liquidity in factory pools.'
      case 'partial':
        return 'Some pools could not be valued. This is a lower bound.'
      case 'verified_empty':
        return 'No liquidity was found in the checked pools.'
      case 'stale':
        return 'The latest pool data is too old.'
      case 'unavailable':
        return 'Pool data is temporarily unavailable.'
      default:
        return 'There is not enough data to value these pools.'
    }
  })()
  const useRetained =
    retained != null &&
    ((quality !== 'fresh' && quality !== 'verified_empty') || retained.asOf !== asOf || retained.value !== raw)
  if (useRetained) {
    value = formatProtocolUsd(retained.value)
    reason = 'Last verified total.'
  }
  const displayAsOf = useRetained ? retained.asOf : asOf
  const displayObserved = displayAsOf ? Date.parse(displayAsOf) : NaN
  const ageMs = now - displayObserved
  const age =
    Number.isFinite(ageMs) && ageMs >= 1_000 && ageMs < 60_000
      ? `${Math.floor(ageMs / 1000)} ${Math.floor(ageMs / 1000) === 1 ? 'second' : 'seconds'} ago`
      : formatRelativeAge(displayAsOf, now)
  const asOfText = age === RELATIVE_AGE_FALLBACK ? 'Update time is unavailable.' : `Updated ${age}.`
  return {
    quality: useRetained ? retained.quality : quality,
    value,
    hint: `${reason} ${asOfText}`,
    // Backend only supplies a delta for matching, qualified baseline coverage.
    comparable: !useRetained && (quality === 'fresh' || quality === 'verified_empty'),
  }
}
