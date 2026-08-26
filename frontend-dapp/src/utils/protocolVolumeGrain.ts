/**
 * Protocol UTC volume chart grain + width clamp (GitLab #668).
 *
 * Bar count follows plot width, then `[min, max]` per grain. Do not request
 * above the indexer allowlist (hourly 168 / daily 90 / monthly 24).
 */

export const PROTOCOL_VOLUME_GRAINS = ['hourly', 'daily', 'monthly'] as const
export type ProtocolVolumeGrain = (typeof PROTOCOL_VOLUME_GRAINS)[number]

export const PROTOCOL_VOLUME_GRAIN_MAX: Record<ProtocolVolumeGrain, number> = {
  hourly: 168,
  daily: 90,
  monthly: 24,
}

export const PROTOCOL_VOLUME_GRAIN_MIN: Record<ProtocolVolumeGrain, number> = {
  hourly: 12,
  daily: 7,
  monthly: 6,
}

/** Plot pixels per bar including gap. ~10–14px keeps phone bars readable. */
export const PROTOCOL_VOLUME_BAR_SLOT_PX = 12

export const PROTOCOL_VOLUME_RESIZE_DEBOUNCE_MS = 160

export function isProtocolVolumeGrain(raw: string): raw is ProtocolVolumeGrain {
  return (PROTOCOL_VOLUME_GRAINS as readonly string[]).includes(raw)
}

export function clampProtocolVolumeLimit(grain: ProtocolVolumeGrain, n: number): number {
  const min = PROTOCOL_VOLUME_GRAIN_MIN[grain]
  const max = PROTOCOL_VOLUME_GRAIN_MAX[grain]
  if (!Number.isFinite(n)) return min
  return Math.min(max, Math.max(min, Math.floor(n)))
}

/** Derive allowlisted `limit` from plot width. */
export function limitFromPlotWidth(widthPx: number, grain: ProtocolVolumeGrain): number {
  if (!Number.isFinite(widthPx) || widthPx <= 0) {
    return PROTOCOL_VOLUME_GRAIN_MIN[grain]
  }
  const slots = Math.floor(widthPx / PROTOCOL_VOLUME_BAR_SLOT_PX)
  return clampProtocolVolumeLimit(grain, slots)
}

export function isAllowlistedProtocolVolumeLimit(grain: ProtocolVolumeGrain, limit: number): boolean {
  return Number.isInteger(limit) && limit >= 1 && limit <= PROTOCOL_VOLUME_GRAIN_MAX[grain]
}

export type ProtocolVolumeSeriesPoint = {
  utc_hour?: string | null
  utc_day?: string | null
  utc_month?: string | null
  volume_usd: string | null
  trade_count: number
}

export function pointPeriod(point: ProtocolVolumeSeriesPoint, grain: ProtocolVolumeGrain): string {
  if (grain === 'hourly') return point.utc_hour ?? ''
  if (grain === 'monthly') return point.utc_month ?? ''
  return point.utc_day ?? ''
}

/** Sparse X-axis labels — do not label every hourly bar. */
export function sparseTimeLabelIndexes(count: number, maxLabels = 5): number[] {
  if (count <= 0) return []
  if (count <= maxLabels) return Array.from({ length: count }, (_, i) => i)
  const out: number[] = []
  const last = count - 1
  for (let i = 0; i < maxLabels; i++) {
    const idx = Math.round((i * last) / (maxLabels - 1))
    if (out[out.length - 1] !== idx) out.push(idx)
  }
  return out
}

export function formatPeriodAxisLabel(period: string, grain: ProtocolVolumeGrain): string {
  if (grain === 'hourly') {
    const t = period.indexOf('T')
    return t >= 0 ? period.slice(t + 1) : period
  }
  if (grain === 'daily' && period.length >= 10) return period.slice(5)
  return period
}

/** 3–5 USD ticks including $0. Peak ≤ 0 → `[0]` only (no divide-by-zero). */
export function usdAxisTicks(peak: number): number[] {
  if (!Number.isFinite(peak) || peak <= 0) return [0]
  return [0, peak * 0.25, peak * 0.5, peak * 0.75, peak]
}

export function maxPricedUsd(points: Array<{ volume_usd: string | null }>): number {
  let max = 0
  for (const p of points) {
    if (p.volume_usd == null || p.volume_usd === '') continue
    const n = Number(p.volume_usd)
    if (Number.isFinite(n) && n > max) max = n
  }
  return max
}
