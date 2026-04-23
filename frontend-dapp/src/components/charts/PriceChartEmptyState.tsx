import type { IndexerPairStats } from '@/types'

const CHART_MIN_HEIGHT = 'min-h-[400px]'

interface PriceChartEmptyStateProps {
  pairStats: IndexerPairStats | undefined
  statsLoading: boolean
}

/**
 * Shown when the indexer returns no usable OHLC rows for the selected interval (successful response, empty or all invalid).
 */
export function PriceChartEmptyState({ pairStats, statsLoading }: PriceChartEmptyStateProps) {
  const closeLine =
    pairStats?.close_price != null && pairStats.close_price !== ''
      ? `24h close (indexer): ${pairStats.close_price}`
      : null

  const ariaLabel =
    'No price chart data for this interval. Try another time range, confirm the indexer has synced, or wait for trades.'

  return (
    <div
      className={`flex flex-col items-center justify-center gap-3 px-4 py-8 ${CHART_MIN_HEIGHT} rounded-lg border border-dashed border-white/15 bg-[repeating-linear-gradient(135deg,transparent,transparent_8px,rgba(255,255,255,0.03)_8px,rgba(255,255,255,0.03)_16px)]`}
      role="img"
      aria-label={ariaLabel}
    >
      <div
        className="flex h-14 w-14 items-center justify-center rounded-full border border-white/10 bg-white/5 text-white/40"
        aria-hidden
      >
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M4 19V5M4 19h16M8 15l3-4 3 2 4-6" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M8 15v4M11 11v8M14 13v6M18 7v12" strokeLinecap="round" />
        </svg>
      </div>
      <div className="text-center max-w-md space-y-2">
        <p className="text-sm font-semibold uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
          No chart data for this interval yet
        </p>
        <p className="text-xs leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
          Try another interval, confirm the indexer has caught up, or wait for trades on this pair.
        </p>
        {statsLoading && (
          <p className="text-[11px] uppercase tracking-wide" style={{ color: 'var(--ink-subtle)' }}>
            Loading reference price…
          </p>
        )}
        {!statsLoading && closeLine && (
          <p className="text-xs font-mono tabular-nums" style={{ color: 'var(--ink-subtle)' }}>
            {closeLine}
          </p>
        )}
      </div>
    </div>
  )
}
