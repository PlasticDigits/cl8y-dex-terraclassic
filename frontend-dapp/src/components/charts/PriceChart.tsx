import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCandles, getPairStats } from '@/services/indexer/client'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { PriceChartEmptyState } from './PriceChartEmptyState'
import { PriceChartLightweightCanvas } from './PriceChartLightweightCanvas'
import {
  applyChartDisplayInvert,
  indexerCandlesToFactoryPoints,
  indexerCandlesToVolumeHistogramPoints,
  type CandleVolumeScale,
} from './priceChartCandles'
import { isPairLegDecimals } from '@/utils/formatAmount'
import { PairDisplayInvertPill } from '@/components/trade/PairDisplayInvertControls'
import { resolveTradeChartHeadlineUsd } from './chartHeadlinePrice'
import { chartPointsToRsiLine, chartPointsToSmaLine } from './priceChartIndicators'
import { PriceChartOverlayMenu } from './PriceChartOverlayMenu'
import { keepPreviousCandlesForIntervalSwitch } from './priceChartCandlesPlaceholder'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { TRADE_PANEL_CHART_UNAVAILABLE } from '@/utils/indexerTradeOutageCopy'
import { TradeMarketDataUnavailableNotice } from '@/components/trade/TradeMarketDataUnavailableNotice'

const INTERVALS = ['1m', '5m', '15m', '1h', '4h', '1d', '1w'] as const

interface PriceChartProps {
  pairAddress: string
  defaultInterval?: string
  /** Latest trade price in USD from indexer tape (newest-first); preferred over candle close for headline */
  tapeLastPriceUsd?: string | null
  /** UI-only invert of factory USD series (GitLab #524). */
  displayInverted?: boolean
  onToggleDisplayInvert?: () => void
  pairPillLabel?: string
  invertAriaLabel?: string
  displayBaseSymbol?: string
  /** Pair-leg decimals for human candle volume (GitLab #564). */
  volumeBaseDecimals?: number
  volumeQuoteDecimals?: number
}

export default function PriceChart({
  pairAddress,
  defaultInterval = '1h',
  tapeLastPriceUsd,
  displayInverted = false,
  onToggleDisplayInvert,
  pairPillLabel,
  invertAriaLabel,
  displayBaseSymbol,
  volumeBaseDecimals,
  volumeQuoteDecimals,
}: PriceChartProps) {
  const panelRef = useRef<HTMLDivElement>(null)
  const chartHeadingId = useId()
  const chartLiveSummaryId = useId()
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
    /** Keep plot mounted on interval switch only — not on pair switch (GitLab #148, #180). */
    placeholderData: (previousData, previousQuery) =>
      keepPreviousCandlesForIntervalSwitch(pairAddress, previousData, previousQuery),
  })

  const chartPoints = useMemo(
    () => applyChartDisplayInvert(indexerCandlesToFactoryPoints(candlesQuery.data), displayInverted),
    [candlesQuery.data, displayInverted]
  )

  const headlineUsd = useMemo(
    () => resolveTradeChartHeadlineUsd(tapeLastPriceUsd, chartPoints),
    [tapeLastPriceUsd, chartPoints]
  )

  const volumeScale = useMemo((): CandleVolumeScale | undefined => {
    if (!isPairLegDecimals(volumeBaseDecimals) || !isPairLegDecimals(volumeQuoteDecimals)) return undefined
    return { baseDecimals: volumeBaseDecimals, quoteDecimals: volumeQuoteDecimals }
  }, [volumeBaseDecimals, volumeQuoteDecimals])

  const volumePoints = useMemo(() => {
    if (!volumeScale) return []
    if (typeof document === 'undefined') {
      return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, '#22c55e', '#ef4444', volumeScale)
    }
    const root = document.documentElement
    const up = getComputedStyle(root).getPropertyValue('--color-positive').trim() || '#22c55e'
    const down = getComputedStyle(root).getPropertyValue('--color-negative').trim() || '#ef4444'
    return indexerCandlesToVolumeHistogramPoints(candlesQuery.data, up, down, volumeScale)
  }, [candlesQuery.data, volumeScale])

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

  const chartIndexerOutage = candlesQuery.isError && isIndexerUnavailableError(candlesQuery.error)

  const chartLiveSummary = useMemo(() => {
    if (showInitialLoading) {
      return `Price chart loading. Interval ${interval}.`
    }
    if (chartIndexerOutage) {
      return `Price chart unavailable. Indexer outage. Interval ${interval}.`
    }
    if (candlesQuery.isError) {
      return `Price chart failed to load. Interval ${interval}.`
    }
    if (showEmptyState) {
      return `Price chart empty. No candles for interval ${interval}.`
    }
    const tokenPart = displayBaseSymbol ? ` for 1 ${displayBaseSymbol}` : ''
    if (intervalRefetching) {
      const pricePart = headlineUsd != null ? ` Last price ${headlineUsd} USD${tokenPart}.` : ''
      return `Price chart updating to interval ${interval}.${pricePart}`
    }
    const pricePart = headlineUsd != null ? ` Last price ${headlineUsd} USD${tokenPart}.` : ' Last price unavailable.'
    const candlePart = chartPoints.length > 0 ? ` ${chartPoints.length} candles on chart.` : ''
    return `Price chart. Interval ${interval}.${pricePart}${candlePart}`
  }, [
    showInitialLoading,
    chartIndexerOutage,
    candlesQuery.isError,
    showEmptyState,
    intervalRefetching,
    interval,
    headlineUsd,
    chartPoints.length,
    displayBaseSymbol,
  ])

  return (
    <section
      ref={panelRef}
      role="region"
      aria-labelledby={chartHeadingId}
      aria-describedby={chartLiveSummaryId}
      className={`shell-panel-strong flex flex-col min-h-0 h-full !overflow-visible ${fsActive ? 'min-h-[100dvh] justify-stretch' : ''}`}
    >
      <p id={chartLiveSummaryId} className="sr-only" aria-live="polite" aria-atomic="true">
        {chartLiveSummary}
      </p>
      <div className="flex flex-col gap-3 mb-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex flex-col gap-1 min-w-0">
            <div className="flex flex-wrap items-baseline gap-2 sm:gap-4">
              <h3
                id={chartHeadingId}
                className="text-sm font-semibold uppercase tracking-wide font-heading"
                style={{ color: 'var(--ink)' }}
              >
                Price (USD)
              </h3>
              {onToggleDisplayInvert && pairPillLabel && (
                <PairDisplayInvertPill
                  label={pairPillLabel}
                  ariaLabel={invertAriaLabel ?? `Show inverted ${pairPillLabel} pricing`}
                  onToggle={onToggleDisplayInvert}
                />
              )}
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
              className="tab-glass !text-[10px] !px-2 !py-1 tab-glass-inactive"
              aria-pressed={fsActive}
              aria-label={fsActive ? 'Exit chart fullscreen' : 'Expand chart to fullscreen'}
            >
              {fsActive ? 'Exit' : 'Expand'}
            </button>
            <div className="flex gap-1 flex-wrap" role="group" aria-label="Chart interval">
              {INTERVALS.map((iv) => (
                <button
                  key={iv}
                  type="button"
                  aria-pressed={interval === iv}
                  aria-label={`${iv} candle interval`}
                  onClick={() => {
                    sounds.playButtonPress()
                    setInterval_(iv)
                  }}
                  className={`tab-glass !text-[10px] !px-2 !py-1 ${interval === iv ? 'tab-glass-active' : 'tab-glass-inactive'}`}
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

      {candlesQuery.isError && chartIndexerOutage && (
        <div className="flex flex-1 items-center justify-center min-h-[min(52vh,280px)] px-3">
          <TradeMarketDataUnavailableNotice
            message={TRADE_PANEL_CHART_UNAVAILABLE}
            data-testid="trade-chart-unavailable"
          />
        </div>
      )}

      {candlesQuery.isError && !chartIndexerOutage && (
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
            key={pairAddress}
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
          <p className="mt-1 text-right shrink-0">
            <a
              href="https://www.tradingview.com/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[10px] hover:underline"
              style={{ color: 'var(--ink-dim)' }}
              data-testid="price-chart-tradingview-attribution"
            >
              Charting by TradingView
            </a>
          </p>
        </div>
      )}

      {showEmptyState && <PriceChartEmptyState pairStats={statsQuery.data} statsLoading={statsQuery.isLoading} />}
    </section>
  )
}
