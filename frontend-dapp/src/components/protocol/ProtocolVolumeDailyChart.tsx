import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { RetryError } from '@/components/ui'
import { formatProtocolUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_FEES_DAILY_LABEL,
  PROTOCOL_FEES_DAILY_TITLE,
  PROTOCOL_FEES_EMPTY,
  PROTOCOL_LIQUIDITY_DAILY_LABEL,
  PROTOCOL_LIQUIDITY_DAILY_TITLE,
  PROTOCOL_LIQUIDITY_EMPTY,
  PROTOCOL_UTC_METRIC_FEES_LABEL,
  PROTOCOL_UTC_METRIC_LIQUIDITY_LABEL,
  PROTOCOL_UTC_METRIC_VOLUME_LABEL,
  PROTOCOL_VOLUME_DAILY_LABEL,
  PROTOCOL_VOLUME_DAILY_TITLE,
  PROTOCOL_VOLUME_EMPTY,
  PROTOCOL_VOLUME_GRAIN_DAILY_LABEL,
  PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL,
  PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL,
  PROTOCOL_VOLUME_GRAIN_SUBTITLE,
} from '@/utils/trailingWindowCopy'
import {
  formatPeriodAxisLabel,
  limitFromPlotWidth,
  maxPricedUsd,
  pointPeriod,
  pointValueUsd,
  PROTOCOL_UTC_METRICS,
  PROTOCOL_VOLUME_GRAINS,
  PROTOCOL_VOLUME_GRAIN_MIN,
  PROTOCOL_VOLUME_RESIZE_DEBOUNCE_MS,
  PROTOCOL_VOLUME_AXIS_PLOT_PX,
  timeLabelIndexes,
  usdAxisTicks,
  type ProtocolUtcMetric,
  type ProtocolVolumeGrain,
  type ProtocolVolumeSeriesPoint,
} from '@/utils/protocolVolumeGrain'
import { isProtocolVolumeSeriesUnavailable, useProtocolUtcSeriesQuery } from './useProtocolVolumeSeriesQuery'

const GRAIN_TAB_LABEL: Record<ProtocolVolumeGrain, string> = {
  hourly: PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL,
  daily: PROTOCOL_VOLUME_GRAIN_DAILY_LABEL,
  monthly: PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL,
}

const METRIC_TAB_LABEL: Record<ProtocolUtcMetric, string> = {
  volume: PROTOCOL_UTC_METRIC_VOLUME_LABEL,
  liquidity: PROTOCOL_UTC_METRIC_LIQUIDITY_LABEL,
  fees: PROTOCOL_UTC_METRIC_FEES_LABEL,
}

const METRIC_CHART_LABEL: Record<ProtocolUtcMetric, string> = {
  volume: PROTOCOL_VOLUME_DAILY_LABEL,
  liquidity: PROTOCOL_LIQUIDITY_DAILY_LABEL,
  fees: PROTOCOL_FEES_DAILY_LABEL,
}

const METRIC_CHART_TITLE: Record<ProtocolUtcMetric, string> = {
  volume: PROTOCOL_VOLUME_DAILY_TITLE,
  liquidity: PROTOCOL_LIQUIDITY_DAILY_TITLE,
  fees: PROTOCOL_FEES_DAILY_TITLE,
}

const METRIC_EMPTY: Record<ProtocolUtcMetric, string> = {
  volume: PROTOCOL_VOLUME_EMPTY,
  liquidity: PROTOCOL_LIQUIDITY_EMPTY,
  fees: PROTOCOL_FEES_EMPTY,
}

const VIEW_H = 128
const PAD_L = 52
const PAD_R = 8
const PAD_T = 8
const PAD_B = 22
const PLOT_H = VIEW_H - PAD_T - PAD_B
/** Floor matches the old letterboxed 320×128 canvas so phone still has a plot. */
const MIN_VIEW_W = PROTOCOL_VOLUME_AXIS_PLOT_PX + PAD_L + PAD_R

function chartAria(metric: ProtocolUtcMetric, grain: ProtocolVolumeGrain): string {
  return `${METRIC_CHART_TITLE[metric]} ${PROTOCOL_VOLUME_GRAIN_SUBTITLE[grain]}.`
}

export function ProtocolVolumeDailyChart() {
  const [metric, setMetric] = useState<ProtocolUtcMetric>('volume')
  const [grain, setGrain] = useState<ProtocolVolumeGrain>('daily')
  const [limit, setLimit] = useState(() => PROTOCOL_VOLUME_GRAIN_MIN.daily)
  const [viewW, setViewW] = useState(MIN_VIEW_W)
  const plotRef = useRef<HTMLDivElement>(null)
  const [active, setActive] = useState<number | null>(null)
  const plotW = Math.max(viewW - PAD_L - PAD_R, 1)

  const applyWidth = useCallback(
    (width: number) => {
      const nextView = Math.max(MIN_VIEW_W, Math.floor(width))
      setViewW((prev) => (prev === nextView ? prev : nextView))
      const next = limitFromPlotWidth(Math.max(width - PAD_L - PAD_R, 0), grain)
      setLimit((prev) => (prev === next ? prev : next))
    },
    [grain]
  )

  useEffect(() => {
    setLimit(PROTOCOL_VOLUME_GRAIN_MIN[grain])
    setActive(null)
  }, [grain])

  useEffect(() => {
    setActive(null)
  }, [metric])

  useEffect(() => {
    const el = plotRef.current
    if (!el || typeof ResizeObserver === 'undefined') return
    let timer: ReturnType<typeof setTimeout> | undefined
    const ro = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? 0
      clearTimeout(timer)
      timer = setTimeout(() => applyWidth(width), PROTOCOL_VOLUME_RESIZE_DEBOUNCE_MS)
    })
    ro.observe(el)
    applyWidth(el.getBoundingClientRect().width)
    return () => {
      clearTimeout(timer)
      ro.disconnect()
    }
  }, [applyWidth])

  const query = useProtocolUtcSeriesQuery(metric, grain, limit)
  const series = useMemo((): ProtocolVolumeSeriesPoint[] => query.data?.series ?? [], [query.data?.series])
  const peak = useMemo(() => maxPricedUsd(series, metric), [series, metric])
  const ticks = useMemo(() => usdAxisTicks(peak), [peak])
  const xLabels = useMemo(() => timeLabelIndexes(series.length, grain, plotW), [series.length, grain, plotW])
  const unavailable = query.isError && isProtocolVolumeSeriesUnavailable(query.error)
  const volumeMissing = metric === 'volume' && unavailable
  const siblingMissing = metric !== 'volume' && unavailable
  const allNull = series.length > 0 && series.every((p) => pointValueUsd(p, metric) == null)
  const empty = !query.isLoading && (series.length === 0 || allNull)

  if (volumeMissing) return null

  const tooltip =
    active != null && series[active]
      ? {
          period: pointPeriod(series[active], grain),
          usd:
            pointValueUsd(series[active], metric) == null
              ? '—'
              : formatProtocolUsd(pointValueUsd(series[active], metric)),
        }
      : null

  const showPlot = !query.isLoading && !query.isError && !empty && !siblingMissing
  const showEmpty = !query.isLoading && !query.isError && empty && !siblingMissing

  return (
    <div
      ref={plotRef}
      className="mt-4 min-w-0 w-full"
      data-testid="protocol-volume-daily-chart"
      data-protocol-utc-series-chart=""
    >
      <div className="flex items-center justify-between gap-2 mb-2 flex-wrap">
        <h3
          className="text-xs font-semibold uppercase tracking-wide"
          style={{ color: 'var(--ink-dim)' }}
          title={METRIC_CHART_TITLE[metric]}
        >
          {METRIC_CHART_LABEL[metric]}
        </h3>
        <div className="flex flex-wrap gap-2 justify-end">
          <div className="flex gap-1" role="tablist" aria-label="UTC chart metric">
            {PROTOCOL_UTC_METRICS.map((m) => (
              <button
                key={m}
                type="button"
                role="tab"
                aria-selected={metric === m}
                data-testid={`protocol-utc-metric-${m}`}
                className="px-2 py-1 text-xs font-medium rounded-md"
                style={{
                  color: metric === m ? 'var(--ink)' : 'var(--ink-dim)',
                  background: metric === m ? 'var(--accent-surface)' : 'transparent',
                }}
                onClick={() => setMetric(m)}
              >
                {METRIC_TAB_LABEL[m]}
              </button>
            ))}
          </div>
          <div className="flex gap-1" role="tablist" aria-label={chartAria(metric, grain)}>
            {PROTOCOL_VOLUME_GRAINS.map((g) => (
              <button
                key={g}
                type="button"
                role="tab"
                aria-selected={grain === g}
                data-testid={`protocol-volume-grain-${g}`}
                className="px-2 py-1 text-xs font-medium rounded-md"
                style={{
                  color: grain === g ? 'var(--ink)' : 'var(--ink-dim)',
                  background: grain === g ? 'var(--accent-surface)' : 'transparent',
                }}
                onClick={() => setGrain(g)}
              >
                {GRAIN_TAB_LABEL[g]}
              </button>
            ))}
          </div>
        </div>
      </div>
      <p className="text-[10px] mb-2" style={{ color: 'var(--ink-dim)' }} title={METRIC_CHART_TITLE[metric]}>
        {PROTOCOL_VOLUME_GRAIN_SUBTITLE[grain]}
      </p>
      {query.isError && !unavailable && (
        <RetryError message={`Failed to load ${metric}`} onRetry={() => void query.refetch()} />
      )}
      {query.isLoading && <div className="h-32 rounded" style={{ background: 'var(--accent-surface)' }} aria-hidden />}
      {showEmpty && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="protocol-volume-daily-empty">
          {METRIC_EMPTY[metric]}
        </p>
      )}
      {showPlot && (
        <div className="relative min-w-0 w-full" data-testid="protocol-volume-plot">
          <svg
            viewBox={`0 0 ${viewW} ${VIEW_H}`}
            width="100%"
            height={VIEW_H}
            preserveAspectRatio="xMinYMid meet"
            className="block w-full h-32"
            role="img"
            aria-label={chartAria(metric, grain)}
            data-testid="protocol-volume-daily-bars"
          >
            <g data-testid="protocol-volume-chart-yaxis">
              {ticks.map((t, i) => {
                const y = PAD_T + PLOT_H - (peak > 0 ? (t / peak) * PLOT_H : 0)
                const label = formatProtocolUsd(t)
                return (
                  <g key={`tick-${i}`}>
                    <line x1={PAD_L} x2={viewW - PAD_R} y1={y} y2={y} stroke="var(--ink-dim)" strokeOpacity={0.25} />
                    <text x={PAD_L - 4} y={y + 3} textAnchor="end" fontSize="8" fill="var(--ink-dim)">
                      {label}
                    </text>
                  </g>
                )
              })}
            </g>
            {series.map((p, i) => {
              const raw = pointValueUsd(p, metric)
              const n = raw == null ? null : Number(raw)
              const priced = n != null && Number.isFinite(n)
              const h = priced && peak > 0 ? Math.max((n / peak) * PLOT_H, n > 0 ? 2 : 0) : 0
              const slot = plotW / Math.max(series.length, 1)
              const x = PAD_L + i * slot
              const w = Math.max(slot * 0.82, 2)
              const period = pointPeriod(p, grain)
              const usd = priced ? formatProtocolUsd(raw) : '—'
              const barLabel = `${period} ${usd}`
              return (
                <rect
                  key={`${period}-${i}`}
                  x={x}
                  y={PAD_T + PLOT_H - h}
                  width={w}
                  height={h}
                  rx={2}
                  tabIndex={0}
                  role="img"
                  aria-label={barLabel}
                  data-testid={`protocol-volume-bar-${i}`}
                  fill={priced ? 'var(--accent)' : 'transparent'}
                  stroke={raw == null ? 'var(--ink-dim)' : active === i ? 'var(--ink)' : 'none'}
                  onMouseEnter={() => setActive(i)}
                  onMouseLeave={() => setActive(null)}
                  onFocus={() => setActive(i)}
                  onBlur={() => setActive(null)}
                />
              )
            })}
            <g data-testid="protocol-volume-chart-xaxis">
              {xLabels.map((idx) => {
                const p = series[idx]
                if (!p) return null
                const period = pointPeriod(p, grain)
                const slot = plotW / Math.max(series.length, 1)
                const x = PAD_L + idx * slot + slot / 2
                return (
                  <text key={`x-${idx}`} x={x} y={VIEW_H - 6} textAnchor="middle" fontSize="8" fill="var(--ink-dim)">
                    {formatPeriodAxisLabel(period, grain)}
                  </text>
                )
              })}
            </g>
          </svg>
          {tooltip && (
            <div
              className="absolute left-1/2 -translate-x-1/2 top-1 px-2 py-1 text-[10px] rounded pointer-events-none"
              style={{ background: 'var(--panel, var(--accent-surface))', color: 'var(--ink)' }}
              role="tooltip"
              data-testid="protocol-volume-chart-tooltip"
            >
              {tooltip.period} {tooltip.usd}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
