import type { IndexerOverview, LiquidityQuality } from '@/types'
import { formatProtocolUsd } from './formatProtocolStats'

/** Reject hostile numeric inputs before they can become USD text or SVG geometry. */
export function validLiquidityUsd(raw: string | null | undefined): string | null {
  if (typeof raw !== 'string' || raw.length > 80 || !/^\d+(?:\.\d+)?$/.test(raw)) return null
  return Number.isFinite(Number(raw)) ? raw : null
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

export function overviewLiquidityDisplay(overview: IndexerOverview | undefined, now = Date.now()) {
  const raw = validLiquidityUsd(overview?.total_liquidity_usd)
  let quality = overview?.liquidity_quality
  const asOf = overview?.liquidity_as_of
  const observed = asOf ? Date.parse(asOf) : NaN
  const ttl = overview?.liquidity_max_age_seconds
  // Volume polling/cache updates must not relabel an older valuation as fresh.
  if (quality === 'fresh' || quality === 'verified_empty' || quality === 'partial') {
    if (!Number.isFinite(observed) || observed > now + 1000) quality = 'unavailable'
    else if (!Number.isFinite(ttl) || ttl == null || ttl <= 0 || now - observed > ttl * 1000) quality = 'stale'
  }
  // A legacy zero does not prove empty reserves, even when the priced count is zero.
  const legacyZero = quality == null && raw != null && Number(raw) === 0
  if (legacyZero || raw == null) quality = 'unavailable'
  quality = liquidityQualityForValue(quality, raw)
  const known = quality === 'fresh' || quality === 'verified_empty' || quality === 'partial' || quality == null
  const value = known && raw != null ? `${quality === 'partial' ? '≥ ' : ''}${formatProtocolUsd(raw)}` : '—'
  const asOfText = Number.isFinite(observed)
    ? `As of ${new Date(observed).toISOString()}`
    : 'Observation time unavailable'
  return {
    quality,
    value,
    hint: `${liquidityQualityLabel(quality)}. ${asOfText}`,
    // Backend only supplies a delta for matching, qualified baseline coverage.
    comparable: quality === 'fresh' || quality === 'verified_empty',
  }
}
