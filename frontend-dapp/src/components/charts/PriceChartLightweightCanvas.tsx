import { useEffect, useRef, useState } from 'react'
import type { AutoscaleInfo, HistogramData, IChartApi, ISeriesApi, Time } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'
import type { IndicatorLinePoint } from './priceChartIndicators'
import { rsiPaneHeightPx, volumePaneHeightPx } from './priceChartPaneHeights'
import { clampUsdPriceChartAutoscale, minLowInVisibleLogicalRange } from './priceChartPriceScale'
import { syncPriceChartIndicatorOverlays, type IndicatorSeriesRefs } from './priceChartLightweightIndicatorSync'
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

      chart.timeScale().fitContent()

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
      sma7Points,
      sma25Points,
      rsiPoints,
      chartHeightPx: h,
    })
    applyLayoutRef.current?.()
    chart.timeScale().fitContent()
  }, [chartModelReady, showSma7, showSma25, showRsi, sma7Points, sma25Points, rsiPoints])

  useEffect(() => {
    if (!candleSeriesRef.current || !volumeSeriesRef.current) return
    candleSeriesRef.current.setData(candlePoints)
    volumeSeriesRef.current.setData(volumePoints)
    if (indicatorRefs.current.sma7) indicatorRefs.current.sma7.setData(sma7Points)
    if (indicatorRefs.current.sma25) indicatorRefs.current.sma25.setData(sma25Points)
    if (indicatorRefs.current.rsi) indicatorRefs.current.rsi.setData(rsiPoints)
    chartRef.current?.timeScale().fitContent()
  }, [candlePoints, volumePoints, sma7Points, sma25Points, rsiPoints])

  return (
    <div
      ref={containerRef}
      className="w-full h-full min-h-[320px]"
      data-testid="price-chart-lightweight-canvas"
      aria-hidden
    />
  )
}
