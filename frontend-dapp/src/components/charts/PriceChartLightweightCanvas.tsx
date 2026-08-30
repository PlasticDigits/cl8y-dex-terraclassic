import { useEffect, useRef, useState } from 'react'
import type { AutoscaleInfo, HistogramData, IChartApi, ISeriesApi, Time } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'
import type { IndicatorLinePoint } from './priceChartIndicators'
import { rsiPaneHeightPx, volumePaneHeightPx } from './priceChartPaneHeights'
import {
  clampUsdPriceChartAutoscale,
  minLowInVisibleLogicalRange,
  usdCandlePriceFormatFromPoints,
} from './priceChartPriceScale'
import { syncPriceChartIndicatorOverlays, type IndicatorSeriesRefs } from './priceChartLightweightIndicatorSync'
import { syncCandleSeriesData, syncHistogramSeriesData, syncLineSeriesData } from './priceChartLightweightSeriesSync'
import { loadPriceChartLightweightModule } from './priceChartLightweightModule'

export interface PriceChartLightweightCanvasProps {
  candlePoints: ChartCandlePoint[]
  volumePoints: HistogramData<Time>[]
  sma7Points: IndicatorLinePoint[]
  sma25Points: IndicatorLinePoint[]
  rsiPoints: IndicatorLinePoint[]
  showSma7: boolean
  showSma25: boolean
  showRsi: boolean
  /** Interval chip value. `fitContent` after interval `setData` only (#705); not on 30s refetch (#336). */
  interval?: string
}

/** TradingView lightweight-charts (open-source); not the hosted TradingView widget product. */
export function PriceChartLightweightCanvas({
  candlePoints,
  volumePoints,
  sma7Points,
  sma25Points,
  rsiPoints,
  showSma7,
  showSma25,
  showRsi,
  interval = '1h',
}: PriceChartLightweightCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const lcRef = useRef<Awaited<typeof import('lightweight-charts')> | null>(null)
  const candleSeriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const volumeSeriesRef = useRef<ISeriesApi<'Histogram'> | null>(null)

  const indicatorRefs = useRef<IndicatorSeriesRefs>({ sma7: null, sma25: null, rsi: null })

  const candlePointsRef = useRef(candlePoints)
  const volumePointsRef = useRef(volumePoints)
  const sma7PointsRef = useRef(sma7Points)
  const sma25PointsRef = useRef(sma25Points)
  const rsiPointsRef = useRef(rsiPoints)
  const showSma7Ref = useRef(showSma7)
  const showSma25Ref = useRef(showSma25)
  const showRsiRef = useRef(showRsi)

  candlePointsRef.current = candlePoints
  volumePointsRef.current = volumePoints
  sma7PointsRef.current = sma7Points
  sma25PointsRef.current = sma25Points
  rsiPointsRef.current = rsiPoints
  showSma7Ref.current = showSma7
  showSma25Ref.current = showSma25
  showRsiRef.current = showRsi

  const applyLayoutRef = useRef<(() => void) | null>(null)
  const chartInitIdRef = useRef(0)
  const previousCandlePointsRef = useRef<ChartCandlePoint[]>([])
  const previousVolumePointsRef = useRef<typeof volumePoints>([])
  const previousSma7PointsRef = useRef<IndicatorLinePoint[]>([])
  const previousSma25PointsRef = useRef<IndicatorLinePoint[]>([])
  const previousRsiPointsRef = useRef<IndicatorLinePoint[]>([])
  const intervalRef = useRef(interval)
  const fittedIntervalRef = useRef<string | null>(null)
  intervalRef.current = interval
  const [chartModelReady, setChartModelReady] = useState(false)

  useEffect(() => {
    const initId = ++chartInitIdRef.current
    let cancelled = false
    let chart: IChartApi | null = null
    let cleanupResize: (() => void) | undefined

    async function initChart() {
      if (!containerRef.current) return

      const lc = await loadPriceChartLightweightModule()
      if (cancelled || initId !== chartInitIdRef.current || !containerRef.current) return

      lcRef.current = lc
      const { CandlestickSeries, HistogramSeries } = lc
      const h = Math.max(320, containerRef.current.clientHeight)
      chart = lc.createChart(containerRef.current, {
        layout: {
          background: { color: 'transparent' },
          textColor: '#9ca3af',
          /** Off-chart link in `PriceChart` satisfies Apache NOTICE; in-chart logo is focusable inside `aria-hidden`. */
          attributionLogo: false,
          panes: {
            enableResize: false,
            separatorColor: 'rgba(255,255,255,0.32)',
            separatorHoverColor: 'rgba(255,255,255,0.2)',
          },
        },
        grid: {
          vertLines: { color: 'rgba(255,255,255,0.04)' },
          horzLines: { color: 'rgba(255,255,255,0.04)' },
        },
        crosshair: {
          mode: 0,
        },
        rightPriceScale: {
          borderColor: 'rgba(255,255,255,0.1)',
        },
        timeScale: {
          borderColor: 'rgba(255,255,255,0.1)',
          timeVisible: true,
          secondsVisible: false,
        },
        width: containerRef.current.clientWidth,
        height: h,
      })

      const positive =
        getComputedStyle(document.documentElement).getPropertyValue('--color-positive').trim() || '#22c55e'
      const negative =
        getComputedStyle(document.documentElement).getPropertyValue('--color-negative').trim() || '#ef4444'

      candleSeriesRef.current = chart.addSeries(
        CandlestickSeries,
        {
          upColor: positive,
          downColor: negative,
          borderDownColor: negative,
          borderUpColor: positive,
          wickDownColor: negative,
          wickUpColor: positive,
          priceFormat: usdCandlePriceFormatFromPoints(candlePointsRef.current),
          autoscaleInfoProvider: (original: () => AutoscaleInfo | null) => {
            const raw = original()
            const logical = chart!.timeScale().getVisibleLogicalRange()
            const visibleMinLow = minLowInVisibleLogicalRange(candlePointsRef.current, logical)
            return clampUsdPriceChartAutoscale(raw, visibleMinLow)
          },
        },
        0
      )
      candleSeriesRef.current.setData(candlePointsRef.current)
      previousCandlePointsRef.current = candlePointsRef.current

      chart.addPane()
      chart.panes()[1]?.setHeight(volumePaneHeightPx(h))

      volumeSeriesRef.current = chart.addSeries(
        HistogramSeries,
        {
          priceFormat: { type: 'volume' },
          color: positive,
          title: 'Volume',
          lastValueVisible: false,
          priceLineVisible: false,
        },
        1
      )
      volumeSeriesRef.current.setData(volumePointsRef.current)
      previousVolumePointsRef.current = volumePointsRef.current

      chart.timeScale().fitContent()
      fittedIntervalRef.current = intervalRef.current

      chartRef.current = chart

      const applySize = () => {
        if (!containerRef.current || !chart) return
        const nextH = Math.max(320, containerRef.current.clientHeight)
        chart.applyOptions({
          width: containerRef.current.clientWidth,
          height: nextH,
        })
        chart.panes()[1]?.setHeight(volumePaneHeightPx(nextH))
        if (showRsiRef.current && chart.panes()[2]) {
          chart.panes()[2]?.setHeight(rsiPaneHeightPx(nextH))
        }
      }
      applyLayoutRef.current = applySize

      const resizeObserver = new ResizeObserver(() => applySize())
      resizeObserver.observe(containerRef.current)
      cleanupResize = () => resizeObserver.disconnect()

      requestAnimationFrame(() => {
        requestAnimationFrame(() => applySize())
      })

      if (!cancelled && initId === chartInitIdRef.current) {
        setChartModelReady(true)
      } else if (chart) {
        cleanupResize?.()
        cleanupResize = undefined
        chart.remove()
        chart = null
        chartRef.current = null
        candleSeriesRef.current = null
        volumeSeriesRef.current = null
        applyLayoutRef.current = null
      }
    }

    void initChart()

    return () => {
      cancelled = true
      if (initId === chartInitIdRef.current) setChartModelReady(false)
      cleanupResize?.()
      applyLayoutRef.current = null
      candleSeriesRef.current = null
      volumeSeriesRef.current = null
      indicatorRefs.current = { sma7: null, sma25: null, rsi: null }
      previousCandlePointsRef.current = []
      previousVolumePointsRef.current = []
      previousSma7PointsRef.current = []
      previousSma25PointsRef.current = []
      previousRsiPointsRef.current = []
      fittedIntervalRef.current = null
      lcRef.current = null
      if (chart) {
        chart.remove()
        chart = null
        chartRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (!chartModelReady || !chartRef.current || !lcRef.current || !containerRef.current) return
    const chart = chartRef.current
    const lc = lcRef.current
    const h = Math.max(320, containerRef.current.clientHeight)

    syncPriceChartIndicatorOverlays(chart, lc, indicatorRefs.current, {
      showSma7,
      showSma25,
      showRsi,
      sma7Points: sma7PointsRef.current,
      sma25Points: sma25PointsRef.current,
      rsiPoints: rsiPointsRef.current,
      chartHeightPx: h,
    })
    if (showSma7 && indicatorRefs.current.sma7) {
      previousSma7PointsRef.current = sma7PointsRef.current
    } else {
      previousSma7PointsRef.current = []
    }
    if (showSma25 && indicatorRefs.current.sma25) {
      previousSma25PointsRef.current = sma25PointsRef.current
    } else {
      previousSma25PointsRef.current = []
    }
    if (showRsi && indicatorRefs.current.rsi) {
      previousRsiPointsRef.current = rsiPointsRef.current
    } else {
      previousRsiPointsRef.current = []
    }
    applyLayoutRef.current?.()
    chart.timeScale().fitContent()
  }, [chartModelReady, showSma7, showSma25, showRsi])

  useEffect(() => {
    if (!candleSeriesRef.current || !volumeSeriesRef.current) return
    const prevFirst = previousCandlePointsRef.current[0]?.time
    const nextFirst = candlePoints[0]?.time
    const intervalChanged = interval !== fittedIntervalRef.current
    candleSeriesRef.current.applyOptions({
      priceFormat: usdCandlePriceFormatFromPoints(candlePoints),
    })
    previousCandlePointsRef.current = syncCandleSeriesData(
      candleSeriesRef.current,
      previousCandlePointsRef.current,
      candlePoints
    )
    previousVolumePointsRef.current = syncHistogramSeriesData(
      volumeSeriesRef.current,
      previousVolumePointsRef.current,
      volumePoints
    )
    if (indicatorRefs.current.sma7) {
      previousSma7PointsRef.current = syncLineSeriesData(
        indicatorRefs.current.sma7,
        previousSma7PointsRef.current,
        sma7Points
      )
    } else {
      previousSma7PointsRef.current = []
    }
    if (indicatorRefs.current.sma25) {
      previousSma25PointsRef.current = syncLineSeriesData(
        indicatorRefs.current.sma25,
        previousSma25PointsRef.current,
        sma25Points
      )
    } else {
      previousSma25PointsRef.current = []
    }
    if (indicatorRefs.current.rsi) {
      previousRsiPointsRef.current = syncLineSeriesData(
        indicatorRefs.current.rsi,
        previousRsiPointsRef.current,
        rsiPoints
      )
    } else {
      previousRsiPointsRef.current = []
    }
    // Interval switch: fit once after the new series `setData`. Placeholder rows keep the
    // previous first-bar time, so we wait until the new interval payload arrives. Sliding
    // newest-N windows change first-bar time at the same interval — do not fit (#336).
    if (intervalChanged && candlePoints.length > 0 && prevFirst !== nextFirst) {
      chartRef.current?.timeScale().fitContent()
      fittedIntervalRef.current = interval
    }
  }, [candlePoints, volumePoints, sma7Points, sma25Points, rsiPoints, interval])

  return (
    <div
      ref={containerRef}
      className="w-full h-full min-h-[320px]"
      data-testid="price-chart-lightweight-canvas"
      aria-hidden
    />
  )
}
