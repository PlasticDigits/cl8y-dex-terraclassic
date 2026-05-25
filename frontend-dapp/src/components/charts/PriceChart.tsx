import { useEffect, useMemo, useRef, useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { getCandles, getPairStats } from '@/services/indexer/client'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { PriceChartEmptyState } from './PriceChartEmptyState'
import { PriceChartLightweightCanvas } from './PriceChartLightweightCanvas'
import { indexerCandlesToChartPoints, indexerCandlesToVolumeHistogramPoints } from './priceChartCandles'
import { resolveTradeChartHeadlineUsd } from './chartHeadlinePrice'
import { chartPointsToRsiLine, chartPointsToSmaLine } from './priceChartIndicators'
import { PriceChartOverlayMenu } from './PriceChartOverlayMenu'

const INTERVALS = ['1m', '5m', '15m', '1h', '4h', '1d', '1w'] as const

interface PriceChartProps {
  pairAddress: string
  defaultInterval?: string
  /** Latest trade price in USD from indexer tape (newest-first); preferred over candle close for headline */
  tapeLastPriceUsd?: string | null
}

export default function PriceChart({ pairAddress, defaultInterval = '1h', tapeLastPriceUsd }: PriceChartProps) {
  const panelRef = useRef<HTMLDivElement>(null)
  const [interval, setInterval_] = useState(defaultInterval)
  const [showSma7, setShowSma7] = useState(false)
  const [showSma25, setShowSma25] = useState(false)
  const [showRsi, setShowRsi] = useState(false)
  const [fsActive, setFsActive] = useState(false)

  useEffect(() => {
    const el = panelRef.current
    const sync = () => setFsActive(el != null && document.fullscreenElement === el)
    document.addEventListener('fullscreenchange', sync)
    return () => document.removeEventListener('fullscreenchange', sync)
  }, [])

  const toggleFullscreen = () => {
    const el = panelRef.current
    if (!el) return
    sounds.playButtonPress()
    void (async () => {
      try {
        if (!document.fullscreenElement) await el.requestFullscreen()
        else await document.exitFullscreen()
      } catch {
        /* unsupported or denied */
      }
    })()
  }

  const candlesQuery = useQuery({
    queryKey: ['candles', pairAddress, interval],
    queryFn: () => getCandles(pairAddress, interval),
    refetchInterval: 30_000,
    enabled: !!pairAddress,
    /** Keep plot mounted while switching intervals — avoids async lightweight-charts re-init races (GitLab #148). */
    placeholderData: keepPreviousData,
  })

  const chartPoints = useMemo(() => indexerCandlesToChartPoints(candlesQuery.data), [candlesQuery.data])

  const headlineUsd = useMemo(
    () => resolveTradeChartHeadlineUsd(tapeLastPriceUsd, chartPoints),
    [tapeLastPriceUsd, chartPoints]
  )

  const volumePoints = useMemo(() => {
    if (typeof document === 'undefined') {
      return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, '#22c55e', '#ef4444')
    }
    const root = document.documentElement
    const up = getComputedStyle(root).getPropertyValue('--color-positive').trim() || '#22c55e'
    const down = getComputedStyle(root).getPropertyValue('--color-negative').trim() || '#ef4444'
    return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, up, down)
  }, [candlesQuery.data])

  const sma7Points = useMemo(() => chartPointsToSmaLine(chartPoints, 7), [chartPoints])
  const sma25Points = useMemo(() => chartPointsToSmaLine(chartPoints, 25), [chartPoints])
  const rsiPoints = useMemo(() => chartPointsToRsiLine(chartPoints, 14), [chartPoints])

  const hasCandlePayload = candlesQuery.data !== undefined
  const showInitialLoading = candlesQuery.isLoading && !hasCandlePayload
  const intervalRefetching = candlesQuery.isFetching && hasCandlePayload && chartPoints.length > 0
  const chartDataResolved = !candlesQuery.isError && hasCandlePayload
  const showEmptyState = chartDataResolved && chartPoints.length === 0 && !candlesQuery.isFetching

  const statsQuery = useQuery({
    queryKey: ['indexer-pair-stats', pairAddress, 'price-chart-empty'],
    queryFn: () => getPairStats(pairAddress),
    enabled: !!pairAddress && showEmptyState,
    staleTime: 60_000,
    retry: false,
  })

  return (
    <div
      ref={panelRef}
      className={`shell-panel-strong flex flex-col min-h-0 h-full !overflow-visible ${fsActive ? 'min-h-[100dvh] justify-stretch' : ''}`}
    >
      <div className="flex flex-col gap-3 mb-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex flex-col gap-1 min-w-0">
            <div className="flex flex-wrap items-baseline gap-2 sm:gap-4">
              <h3
                className="text-sm font-semibold uppercase tracking-wide font-heading"
                style={{ color: 'var(--ink)' }}
              >
                Price (USD)
              </h3>
              {headlineUsd != null && (
                <div
                  className="flex items-baseline gap-2"
                  data-testid="trade-chart-headline-price"
                  title="Last trade price (USD) from the tape when available; otherwise last candle close for this interval."
                >
                  <span
                    className="text-[10px] font-semibold uppercase tracking-wide"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    Last
                  </span>
                  <span className="text-lg font-semibold tabular-nums font-heading" style={{ color: 'var(--ink)' }}>
                    {headlineUsd}
                  </span>
                </div>
              )}
            </div>
            <p className="text-[10px] uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>
              Volume (quote, else base)
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2 justify-end ml-auto">
            <PriceChartOverlayMenu
              showSma7={showSma7}
              showSma25={showSma25}
              showRsi={showRsi}
              onToggleSma7={() => {
                setShowSma7((v) => !v)
              }}
              onToggleSma25={() => {
                setShowSma25((v) => !v)
              }}
              onToggleRsi={() => {
                setShowRsi((v) => !v)
              }}
            />
            <button
              type="button"
              data-testid="price-chart-fullscreen"
              onClick={toggleFullscreen}
              className="tab-neo !text-[10px] !px-2 !py-1 tab-neo-inactive"
              aria-pressed={fsActive}
              aria-label={fsActive ? 'Exit chart fullscreen' : 'Expand chart to fullscreen'}
            >
              {fsActive ? 'Exit' : 'Expand'}
            </button>
            <div className="flex gap-1 flex-wrap" role="group" aria-label="Chart interval">
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
        </div>
      </div>

      {showInitialLoading && (
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
        <div
          className={`relative w-full flex-1 min-h-0 flex flex-col ${
            fsActive ? 'min-h-[calc(100dvh-11rem)] h-[calc(100dvh-11rem)]' : 'min-h-[min(52vh,280px)]'
          }`}
        >
          <PriceChartLightweightCanvas
            candlePoints={chartPoints}
            volumePoints={volumePoints}
            sma7Points={sma7Points}
            sma25Points={sma25Points}
            rsiPoints={rsiPoints}
            showSma7={showSma7}
            showSma25={showSma25}
            showRsi={showRsi}
          />
          {intervalRefetching && (
            <div
              className="absolute inset-0 z-10 flex items-center justify-center gap-2 bg-black/35 pointer-events-none"
              data-testid="price-chart-interval-loading"
              aria-live="polite"
              aria-busy="true"
            >
              <Spinner />
              <span
                className="text-[10px] font-semibold uppercase tracking-wide"
                style={{ color: 'var(--ink-subtle)' }}
              >
                Updating interval…
              </span>
            </div>
          )}
        </div>
      )}

      {showEmptyState && <PriceChartEmptyState pairStats={statsQuery.data} statsLoading={statsQuery.isLoading} />}
    </div>
  )
}
