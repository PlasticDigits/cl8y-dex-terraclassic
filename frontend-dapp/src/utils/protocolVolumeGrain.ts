/**
 * Protocol UTC volume chart grain + width clamp (GitLab #668 / #677 / #703).
 *
 * Bar count follows plot width, then `[min, max]` per grain. Do not request
 * above the indexer allowlist (hourly 168 / daily 90 / monthly 24).
 * Monthly on narrow plots caps at 12 (or 6) so horizontal `YY-MM` ticks fit.
 * X-axis labels use step 1 or 2 (hourly may widen when step 2 collides).
 * Do not rotate tick text.
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

/**
 * Preferred monthly bar count on plots too narrow for 24 × `YY-MM` (GitLab #703).
 * Drop to `PROTOCOL_VOLUME_GRAIN_MIN.monthly` (6) when 12 still collides.
 */
export const PROTOCOL_VOLUME_MONTHLY_NARROW_MAX = 12

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

/**
 * Monthly `limit` so horizontal `YY-MM` ticks do not overlap, including the
 * adjacent last-two labels that `timeLabelIndexes` always keeps (GitLab #703).
 * Slot math alone still yields 24 months on a ~260–320px phone plot.
 */
function monthlyLimitForPlotWidth(widthPx: number): number {
  const slotBased = clampProtocolVolumeLimit('monthly', Math.floor(widthPx / PROTOCOL_VOLUME_BAR_SLOT_PX))
  const labelW = estimatedAxisLabelWidthPx('monthly')
  const fits = (n: number) => n >= 1 && widthPx / n >= labelW
  if (fits(PROTOCOL_VOLUME_GRAIN_MAX.monthly)) {
    return Math.min(slotBased, PROTOCOL_VOLUME_GRAIN_MAX.monthly)
  }
  if (fits(PROTOCOL_VOLUME_MONTHLY_NARROW_MAX)) {
    return Math.min(slotBased, PROTOCOL_VOLUME_MONTHLY_NARROW_MAX)
  }
  return PROTOCOL_VOLUME_GRAIN_MIN.monthly
}

/** Derive allowlisted `limit` from plot width. */
export function limitFromPlotWidth(widthPx: number, grain: ProtocolVolumeGrain): number {
  if (!Number.isFinite(widthPx) || widthPx <= 0) {
    return PROTOCOL_VOLUME_GRAIN_MIN[grain]
  }
  if (grain === 'monthly') {
    return monthlyLimitForPlotWidth(widthPx)
  }
  const slots = Math.floor(widthPx / PROTOCOL_VOLUME_BAR_SLOT_PX)
  return clampProtocolVolumeLimit(grain, slots)
}

export function isAllowlistedProtocolVolumeLimit(grain: ProtocolVolumeGrain, limit: number): boolean {
  return Number.isInteger(limit) && limit >= 1 && limit <= PROTOCOL_VOLUME_GRAIN_MAX[grain]
}

export const PROTOCOL_UTC_METRICS = ['volume', 'liquidity', 'fees'] as const
export type ProtocolUtcMetric = (typeof PROTOCOL_UTC_METRICS)[number]

export function isProtocolUtcMetric(raw: string): raw is ProtocolUtcMetric {
  return (PROTOCOL_UTC_METRICS as readonly string[]).includes(raw)
}

export type ProtocolVolumeSeriesPoint = {
  utc_hour?: string | null
  utc_day?: string | null
  utc_month?: string | null
  volume_usd?: string | null
  liquidity_usd?: string | null
  fees_usd?: string | null
  trade_count?: number
  priced_pair_count?: number
  event_count?: number
}

export function pointValueUsd(point: ProtocolVolumeSeriesPoint, metric: ProtocolUtcMetric = 'volume'): string | null {
  if (metric === 'liquidity') return point.liquidity_usd ?? null
  if (metric === 'fees') return point.fees_usd ?? null
  return point.volume_usd ?? null
}

export function pointPeriod(point: ProtocolVolumeSeriesPoint, grain: ProtocolVolumeGrain): string {
  if (grain === 'hourly') return point.utc_hour ?? ''
  if (grain === 'monthly') return point.utc_month ?? ''
  return point.utc_day ?? ''
}

/**
 * Default plot width used when ResizeObserver has not measured yet
 * (320 − padL 52 − padR 8). Live charts set viewBox width from the panel.
 */
export const PROTOCOL_VOLUME_AXIS_PLOT_PX = 260

/** SVG `fontSize="8"` ≈ 0.6em per glyph, plus a small gap. */
const AXIS_LABEL_EM_PX = 4.8
const AXIS_LABEL_PAD_PX = 2

/** Formatted axis width: hourly `HH`, daily `MM-DD`, monthly `YY-MM` (#703). */
export function estimatedAxisLabelWidthPx(grain: ProtocolVolumeGrain): number {
  const chars = grain === 'hourly' ? 2 : 5
  return chars * AXIS_LABEL_EM_PX + AXIS_LABEL_PAD_PX
}

/**
 * X-axis step (GitLab #677 / **P668-9**).
 * Daily and Monthly: 1 (every bar) or 2 (every other). Hourly may use a wider
 * step only when step 2 still collides in the **current** plot width. No global maxLabels=5.
 */
export function timeLabelStep(
  count: number,
  grain: ProtocolVolumeGrain,
  plotWidthPx: number = PROTOCOL_VOLUME_AXIS_PLOT_PX
): number {
  if (count <= 1) return 1
  const slot = plotWidthPx / count
  if (!Number.isFinite(slot) || slot <= 0) return 1
  const raw = Math.max(1, Math.ceil(estimatedAxisLabelWidthPx(grain) / slot))
  if (grain === 'hourly') return raw
  return raw <= 1 ? 1 : 2
}

/** First and last indexes stay labeled when any labels are shown. */
export function timeLabelIndexes(
  count: number,
  grain: ProtocolVolumeGrain,
  plotWidthPx: number = PROTOCOL_VOLUME_AXIS_PLOT_PX
): number[] {
  if (count <= 0) return []
  if (count === 1) return [0]
  const step = timeLabelStep(count, grain, plotWidthPx)
  const last = count - 1
  const out: number[] = []
  for (let i = 0; i <= last; i += step) out.push(i)
  if (out[out.length - 1] !== last) out.push(last)
  return out
}

/**
 * Axis tick text. Monthly is `YY-MM` (GitLab #703); tooltip / `aria-label`
 * keep the full period via `pointPeriod`. Fail-closed to a short string —
 * never throw on malformed / XSS-like `utc_month`.
 */
export function formatPeriodAxisLabel(period: string, grain: ProtocolVolumeGrain): string {
  if (grain === 'hourly') {
    const t = period.indexOf('T')
    return t >= 0 ? period.slice(t + 1) : period
  }
  if (grain === 'daily' && period.length >= 10) return period.slice(5)
  if (grain === 'monthly') {
    const m = /^(\d{2}|\d{4})-(\d{2})$/.exec(period.trim())
    if (m) {
      const yy = m[1].length === 4 ? m[1].slice(2) : m[1]
      return `${yy}-${m[2]}`
    }
    const compact = period.replace(/\s+/g, ' ').trim()
    if (!compact) return '—'
    return compact.length > 5 ? compact.slice(0, 5) : compact
  }
  return period
}

/** 3–5 USD ticks including $0. Peak ≤ 0 → `[0]` only (no divide-by-zero). */
export function usdAxisTicks(peak: number): number[] {
  if (!Number.isFinite(peak) || peak <= 0) return [0]
  return [0, peak * 0.25, peak * 0.5, peak * 0.75, peak]
}

export function maxPricedUsd(
  points: Array<{ volume_usd?: string | null; liquidity_usd?: string | null; fees_usd?: string | null }>,
  metric: ProtocolUtcMetric = 'volume'
): number {
  let max = 0
  for (const p of points) {
    const raw = pointValueUsd(p, metric)
    if (raw == null || raw === '') continue
    const n = Number(raw)
    if (Number.isFinite(n) && n > max) max = n
  }
  return max
}
