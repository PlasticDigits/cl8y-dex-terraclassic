import { useNumberDisplayMode } from '@/utils/numberDisplayPreference'
import { useCallback, useEffect, useMemo, useRef, useState, useId } from 'react'
import { RetryError } from '@/components/ui'
import { ProtocolDetails } from './ProtocolDetails'
import { formatProtocolUsd } from '@/utils/formatProtocolStats'
import { liquidityQualityForValue, liquidityQualityLabel, validLiquidityUsd } from '@/utils/protocolLiquidityQuality'
import {
  PROTOCOL_FEES_DAILY_TITLE,
  PROTOCOL_FEES_EMPTY,
  PROTOCOL_LIQUIDITY_DAILY_TITLE,
  PROTOCOL_LIQUIDITY_EMPTY,
  PROTOCOL_UTC_METRIC_FEES_LABEL,
  PROTOCOL_UTC_METRIC_LIQUIDITY_LABEL,
  PROTOCOL_UTC_METRIC_VOLUME_LABEL,
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
  maxPlotUsd,
  historicalLiquidityEstimateUsd,
  plotPointValueUsd,
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
  useNumberDisplayMode()
  const hatchId = `protocol-estimate-${useId().replace(/:/g, '')}`
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
  const peak = useMemo(() => maxPlotUsd(series, metric), [series, metric])
  const ticks = useMemo(() => usdAxisTicks(peak), [peak])
  const xLabels = useMemo(() => timeLabelIndexes(series.length, grain, plotW), [series.length, grain, plotW])
  const unavailable = query.isError && isProtocolVolumeSeriesUnavailable(query.error)
  const volumeMissing = metric === 'volume' && unavailable
  const siblingMissing = metric !== 'volume' && unavailable
  const allNull = series.length > 0 && series.every((p) => plotPointValueUsd(p, metric) == null)
  const empty = !query.isLoading && (series.length === 0 || (allNull && metric !== 'liquidity'))
  const pointDescription = (point: ProtocolVolumeSeriesPoint) => {
    if (metric !== 'liquidity') return ''
    if (historicalLiquidityEstimateUsd(point) != null) return 'Historical estimate'
    const quality = liquidityQualityForValue(point.quality, validLiquidityUsd(point.liquidity_usd))
    const partial = quality === 'partial' ? validLiquidityUsd(point.liquidity_usd) : null
    const label =
      pointValueUsd(point, metric) == null && !quality ? 'Unknown liquidity' : liquidityQualityLabel(quality)
    return `${label}${partial != null ? `: ≥ ${formatProtocolUsd(partial)}` : ''}${point.as_of ? ` · As of ${point.as_of}` : ''}`
  }

  if (volumeMissing) return null

  const tooltip =
    active != null && series[active]
      ? {
          period: pointPeriod(series[active], grain),
          usd:
            plotPointValueUsd(series[active], metric) == null
              ? '—'
              : `${metric === 'liquidity' && historicalLiquidityEstimateUsd(series[active]) != null ? '≈ ' : ''}${formatProtocolUsd(plotPointValueUsd(series[active], metric))}`,
          quality: pointDescription(series[active]),
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
          {metric === 'liquidity' && (
            <ProtocolDetails
              label="Chart details"
              detail="Hatched bars are historical estimates. Gaps mean unavailable data."
            />
          )}
        </div>
      </div>
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
            <defs>
              <pattern id={hatchId} width="5" height="5" patternUnits="userSpaceOnUse">
                <rect width="5" height="5" fill="var(--ink-dim)" fillOpacity="0.12" />
                <path d="M-1 1L1-1M0 5L5 0M4 6L6 4" stroke="var(--ink-dim)" strokeOpacity="0.6" />
              </pattern>
            </defs>
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
              const estimate = metric === 'liquidity' && historicalLiquidityEstimateUsd(p) != null
              const raw = plotPointValueUsd(p, metric)
              const n = raw == null ? null : Number(raw)
              const priced = n != null && Number.isFinite(n)
              const h = priced && peak > 0 ? Math.max((n / peak) * PLOT_H, n > 0 ? 2 : 0) : 0
              const slot = plotW / Math.max(series.length, 1)
              const x = PAD_L + i * slot
              const w = Math.max(slot * 0.82, 2)
              const period = pointPeriod(p, grain)
              const usd = priced ? `${estimate ? '≈ ' : ''}${formatProtocolUsd(raw)}` : '—'
              const gap = metric === 'liquidity' && !priced
              const barLabel = `${period} ${usd}${metric === 'liquidity' ? ` · ${pointDescription(p)}` : ''}`
              return (
                <rect
                  key={`${period}-${i}`}
                  x={x}
                  y={gap ? PAD_T : PAD_T + PLOT_H - h}
                  width={w}
                  height={gap ? PLOT_H : h}
                  rx={2}
                  tabIndex={0}
                  role="img"
                  aria-label={barLabel}
                  aria-describedby={active === i ? 'protocol-volume-chart-tooltip' : undefined}
                  data-testid={`protocol-volume-bar-${i}`}
                  fill={estimate ? `url(#${hatchId})` : priced ? 'var(--accent)' : 'transparent'}
                  data-historical-estimate={estimate ? 'true' : undefined}
                  stroke={estimate || raw == null ? 'var(--ink-dim)' : active === i ? 'var(--ink)' : 'none'}
                  strokeDasharray={gap ? '3 3' : estimate ? '2 2' : undefined}
                  data-quality={
                    metric === 'liquidity'
                      ? (liquidityQualityForValue(p.quality, validLiquidityUsd(p.liquidity_usd)) ?? 'unknown')
                      : undefined
                  }
                  onClick={() => setActive(i)}
                  onKeyDown={(event) => {
                    if (event.key === 'Escape') setActive(null)
                  }}
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
                  <text
                    key={`x-${idx}`}
                    x={x}
                    y={VIEW_H - 6}
                    textAnchor="middle"
                    fontSize="8"
                    fill="var(--ink-dim)"
                    style={{ transform: 'none' }}
                  >
                    {formatPeriodAxisLabel(period, grain)}
                  </text>
                )
              })}
            </g>
          </svg>
          {tooltip && (
            <div
              className="absolute left-1/2 -translate-x-1/2 top-1 max-w-full break-words px-2 py-1 text-[10px] rounded pointer-events-none"
              style={{ background: 'var(--panel, var(--accent-surface))', color: 'var(--ink)' }}
              role="tooltip"
              id="protocol-volume-chart-tooltip"
              data-testid="protocol-volume-chart-tooltip"
            >
              {tooltip.period} {tooltip.usd} {tooltip.quality}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
