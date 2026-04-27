import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCandles, getPairStats } from '@/services/indexer/client'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { PriceChartEmptyState } from './PriceChartEmptyState'
import { PriceChartLightweightCanvas } from './PriceChartLightweightCanvas'
import { indexerCandlesToChartPoints, indexerCandlesToVolumeHistogramPoints } from './priceChartCandles'

const INTERVALS = ['1m', '5m', '15m', '1h', '4h', '1d', '1w'] as const

interface PriceChartProps {
  pairAddress: string
  defaultInterval?: string
}

export default function PriceChart({ pairAddress, defaultInterval = '1h' }: PriceChartProps) {
  const [interval, setInterval_] = useState(defaultInterval)

  const candlesQuery = useQuery({
    queryKey: ['candles', pairAddress, interval],
    queryFn: () => getCandles(pairAddress, interval),
    refetchInterval: 30_000,
    enabled: !!pairAddress,
  })

  const chartPoints = useMemo(() => indexerCandlesToChartPoints(candlesQuery.data), [candlesQuery.data])

  const volumePoints = useMemo(() => {
    if (typeof document === 'undefined') {
      return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, '#22c55e', '#ef4444')
    }
    const root = document.documentElement
    const up = getComputedStyle(root).getPropertyValue('--color-positive').trim() || '#22c55e'
    const down = getComputedStyle(root).getPropertyValue('--color-negative').trim() || '#ef4444'
    return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, up, down)
  }, [candlesQuery.data])

  const chartDataResolved = !candlesQuery.isLoading && !candlesQuery.isError && candlesQuery.isSuccess
  const showEmptyState = chartDataResolved && chartPoints.length === 0

  const statsQuery = useQuery({
    queryKey: ['indexer-pair-stats', pairAddress, 'price-chart-empty'],
    queryFn: () => getPairStats(pairAddress),
    enabled: !!pairAddress && showEmptyState,
    staleTime: 60_000,
    retry: false,
  })

  return (
    <div className="shell-panel-strong flex flex-col min-h-0">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
          Price (USD)
        </h3>
        <div className="flex gap-1" role="group" aria-label="Chart interval">
          {INTERVALS.map((iv) => (
            <button
              key={iv}
              aria-pressed={interval === iv}
              onClick={() => {
                sounds.playButtonPress()
                setInterval_(iv)
              }}
              className={`tab-neo !text-[10px] !px-2 !py-1 ${interval === iv ? 'tab-neo-active' : 'tab-neo-inactive'}`}
            >
              {iv}
            </button>
          ))}
        </div>
      </div>

      {candlesQuery.isLoading && (
        <div className="flex items-center justify-center min-h-[400px] gap-3" style={{ color: 'var(--ink-subtle)' }}>
          <Spinner /> Loading chart...
        </div>
      )}

      {candlesQuery.isError && (
        <div className="flex items-center justify-center min-h-[400px] text-red-400 text-sm uppercase tracking-wide font-semibold">
          Failed to load chart data
        </div>
      )}

      {chartDataResolved && chartPoints.length > 0 && (
        <div className="relative w-full min-h-[min(52vh,560px)] flex-1">
          <PriceChartLightweightCanvas candlePoints={chartPoints} volumePoints={volumePoints} />
        </div>
      )}

      {showEmptyState && <PriceChartEmptyState pairStats={statsQuery.data} statsLoading={statsQuery.isLoading} />}
    </div>
  )
}
