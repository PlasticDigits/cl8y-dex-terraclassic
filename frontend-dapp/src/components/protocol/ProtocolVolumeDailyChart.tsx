import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { RetryError } from '@/components/ui'
import { formatProtocolUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_VOLUME_DAILY_LABEL,
  PROTOCOL_VOLUME_DAILY_TITLE,
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
  PROTOCOL_VOLUME_GRAINS,
  PROTOCOL_VOLUME_GRAIN_MIN,
  PROTOCOL_VOLUME_RESIZE_DEBOUNCE_MS,
  timeLabelIndexes,
  usdAxisTicks,
  type ProtocolVolumeGrain,
} from '@/utils/protocolVolumeGrain'
import { isProtocolVolumeSeriesUnavailable, useProtocolVolumeSeriesQuery } from './useProtocolVolumeSeriesQuery'

const GRAIN_TAB_LABEL: Record<ProtocolVolumeGrain, string> = {
  hourly: PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL,
  daily: PROTOCOL_VOLUME_GRAIN_DAILY_LABEL,
  monthly: PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL,
}

const VIEW_W = 320
const VIEW_H = 128
const PAD_L = 52
const PAD_R = 8
const PAD_T = 8
const PAD_B = 22
const PLOT_W = VIEW_W - PAD_L - PAD_R
const PLOT_H = VIEW_H - PAD_T - PAD_B

function grainAria(grain: ProtocolVolumeGrain): string {
  return `${PROTOCOL_VOLUME_DAILY_TITLE} ${PROTOCOL_VOLUME_GRAIN_SUBTITLE[grain]}.`
}

export function ProtocolVolumeDailyChart() {
  const [grain, setGrain] = useState<ProtocolVolumeGrain>('daily')
  const [limit, setLimit] = useState(() => PROTOCOL_VOLUME_GRAIN_MIN.daily)
  const plotRef = useRef<HTMLDivElement>(null)
  const [active, setActive] = useState<number | null>(null)

  const applyWidth = useCallback(
    (width: number) => {
      const next = limitFromPlotWidth(width, grain)
      setLimit((prev) => (prev === next ? prev : next))
    },
    [grain]
  )

  useEffect(() => {
    setLimit(PROTOCOL_VOLUME_GRAIN_MIN[grain])
    setActive(null)
  }, [grain])

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

  const query = useProtocolVolumeSeriesQuery(grain, limit)
  const series = useMemo(() => query.data?.series ?? [], [query.data?.series])
  const peak = useMemo(() => maxPricedUsd(series), [series])
  const ticks = useMemo(() => usdAxisTicks(peak), [peak])
  const xLabels = useMemo(() => timeLabelIndexes(series.length, grain, PLOT_W), [series.length, grain])
  const unavailable = query.isError && isProtocolVolumeSeriesUnavailable(query.error)
  const allNull = series.length > 0 && series.every((p) => p.volume_usd == null)
  const empty = !query.isLoading && (series.length === 0 || allNull)

  if (unavailable) return null

  const tooltip =
    active != null && series[active]
      ? {
          period: pointPeriod(series[active], grain),
          usd: series[active].volume_usd == null ? '—' : formatProtocolUsd(series[active].volume_usd),
        }
      : null

  return (
    <div className="mt-4 min-w-0" data-testid="protocol-volume-daily-chart">
      <div className="flex items-center justify-between gap-2 mb-2 flex-wrap">
        <h3
          className="text-xs font-semibold uppercase tracking-wide"
          style={{ color: 'var(--ink-dim)' }}
          title={PROTOCOL_VOLUME_DAILY_TITLE}
        >
          {PROTOCOL_VOLUME_DAILY_LABEL}
        </h3>
        <div className="flex gap-1" role="tablist" aria-label={grainAria(grain)}>
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
      <p className="text-[10px] mb-2" style={{ color: 'var(--ink-dim)' }} title={PROTOCOL_VOLUME_DAILY_TITLE}>
        {PROTOCOL_VOLUME_GRAIN_SUBTITLE[grain]}
      </p>
      {query.isError && <RetryError message="Failed to load volume" onRetry={() => void query.refetch()} />}
      {query.isLoading && <div className="h-32 rounded" style={{ background: 'var(--accent-surface)' }} aria-hidden />}
      {!query.isLoading && !query.isError && empty && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="protocol-volume-daily-empty">
          No volume yet
        </p>
      )}
      {!query.isLoading && !query.isError && !empty && (
        <div className="relative min-w-0" ref={plotRef} data-testid="protocol-volume-plot">
          <svg
            viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
            className="w-full h-32"
            role="img"
            aria-label={grainAria(grain)}
            data-testid="protocol-volume-daily-bars"
          >
            <g data-testid="protocol-volume-chart-yaxis">
              {ticks.map((t, i) => {
                const y = PAD_T + PLOT_H - (peak > 0 ? (t / peak) * PLOT_H : 0)
                const label = formatProtocolUsd(t)
                return (
                  <g key={`tick-${i}`}>
                    <line x1={PAD_L} x2={VIEW_W - PAD_R} y1={y} y2={y} stroke="var(--ink-dim)" strokeOpacity={0.25} />
                    <text x={PAD_L - 4} y={y + 3} textAnchor="end" fontSize="8" fill="var(--ink-dim)">
                      {label}
                    </text>
                  </g>
                )
              })}
            </g>
            {series.map((p, i) => {
              const n = p.volume_usd == null ? null : Number(p.volume_usd)
              const priced = n != null && Number.isFinite(n)
              const h = priced && peak > 0 ? Math.max((n / peak) * PLOT_H, n > 0 ? 2 : 0) : 0
              const slot = PLOT_W / Math.max(series.length, 1)
              const x = PAD_L + i * slot
              const w = Math.max(slot - 3, 2)
              const period = pointPeriod(p, grain)
              const usd = priced ? formatProtocolUsd(p.volume_usd) : '—'
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
                  stroke={p.volume_usd == null ? 'var(--ink-dim)' : active === i ? 'var(--ink)' : 'none'}
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
                const slot = PLOT_W / Math.max(series.length, 1)
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
