import { useMemo } from 'react'
import { RetryError } from '@/components/ui'
import { formatProtocolUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_VOLUME_DAILY_7D_LABEL,
  PROTOCOL_VOLUME_DAILY_30D_LABEL,
  PROTOCOL_VOLUME_DAILY_LABEL,
  PROTOCOL_VOLUME_DAILY_TITLE,
} from '@/utils/trailingWindowCopy'
import type { ProtocolVolumeDailyPoint } from '@/types'
import { isProtocolVolumeDailyUnavailable, useProtocolVolumeDailyQuery } from './useProtocolVolumeDailyQuery'

interface ProtocolVolumeDailyChartProps {
  days: 7 | 30
  onDaysChange: (days: 7 | 30) => void
}

function maxUsd(points: ProtocolVolumeDailyPoint[]): number {
  let max = 0
  for (const p of points) {
    if (p.volume_usd == null || p.volume_usd === '') continue
    const n = Number(p.volume_usd)
    if (Number.isFinite(n) && n > max) max = n
  }
  return max
}

export function ProtocolVolumeDailyChart({ days, onDaysChange }: ProtocolVolumeDailyChartProps) {
  const query = useProtocolVolumeDailyQuery(days)
  const series = useMemo(() => query.data?.series ?? [], [query.data?.series])
  const peak = useMemo(() => maxUsd(series), [series])
  const unavailable = query.isError && isProtocolVolumeDailyUnavailable(query.error)
  const allNull = series.length > 0 && series.every((p) => p.volume_usd == null)
  const empty = !query.isLoading && (series.length === 0 || allNull)

  if (unavailable) return null

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
        <div className="flex gap-1" role="tablist" aria-label={PROTOCOL_VOLUME_DAILY_TITLE}>
          {([7, 30] as const).map((n) => (
            <button
              key={n}
              type="button"
              role="tab"
              aria-selected={days === n}
              data-testid={`protocol-volume-daily-${n}d`}
              className="px-2 py-1 text-xs font-medium rounded-md"
              style={{
                color: days === n ? 'var(--ink)' : 'var(--ink-dim)',
                background: days === n ? 'var(--accent-surface)' : 'transparent',
              }}
              onClick={() => onDaysChange(n)}
            >
              {n === 7 ? PROTOCOL_VOLUME_DAILY_7D_LABEL : PROTOCOL_VOLUME_DAILY_30D_LABEL}
            </button>
          ))}
        </div>
      </div>
      <p className="text-[10px] mb-2" style={{ color: 'var(--ink-dim)' }} title={PROTOCOL_VOLUME_DAILY_TITLE}>
        UTC calendar day
      </p>
      {query.isError && <RetryError message="Failed to load daily volume" onRetry={() => void query.refetch()} />}
      {query.isLoading && <div className="h-24 rounded" style={{ background: 'var(--accent-surface)' }} aria-hidden />}
      {!query.isLoading && !query.isError && empty && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="protocol-volume-daily-empty">
          No daily volume yet
        </p>
      )}
      {!query.isLoading && !query.isError && !empty && (
        <svg
          viewBox="0 0 320 96"
          className="w-full h-24"
          role="img"
          aria-label={PROTOCOL_VOLUME_DAILY_TITLE}
          data-testid="protocol-volume-daily-bars"
        >
          {series.map((p, i) => {
            const n = p.volume_usd == null ? null : Number(p.volume_usd)
            const priced = n != null && Number.isFinite(n)
            const h = priced && peak > 0 ? Math.max((n / peak) * 80, n > 0 ? 2 : 0) : 0
            const x = 8 + i * (304 / Math.max(series.length, 1))
            const w = Math.max(304 / Math.max(series.length, 1) - 4, 2)
            const label = priced ? formatProtocolUsd(p.volume_usd) : '—'
            return (
              <g key={p.utc_day}>
                <title>{`${p.utc_day} ${label}`}</title>
                <rect
                  x={x}
                  y={88 - h}
                  width={w}
                  height={h}
                  rx={2}
                  fill={priced ? 'var(--accent)' : 'transparent'}
                  stroke={p.volume_usd == null ? 'var(--ink-dim)' : 'none'}
                />
              </g>
            )
          })}
        </svg>
      )}
    </div>
  )
}
