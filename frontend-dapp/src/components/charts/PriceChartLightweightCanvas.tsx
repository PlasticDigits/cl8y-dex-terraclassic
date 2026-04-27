import { useEffect, useRef } from 'react'
import { CandlestickSeries, HistogramSeries } from 'lightweight-charts'
import type { HistogramData, IChartApi, ISeriesApi, Time } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'

/** Short fixed strip at the bottom of the chart (px); scales slightly with chart height. */
function volumePaneHeightPx(totalChartHeight: number): number {
  return Math.min(52, Math.max(32, Math.round(totalChartHeight * 0.042)))
}

interface PriceChartLightweightCanvasProps {
  candlePoints: ChartCandlePoint[]
  volumePoints: HistogramData<Time>[]
}

/** Mounts TradingView lightweight-charts only when there is at least one valid OHLC point. */
export function PriceChartLightweightCanvas({ candlePoints, volumePoints }: PriceChartLightweightCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const volumeSeriesRef = useRef<ISeriesApi<'Histogram'> | null>(null)
  const candlePointsRef = useRef(candlePoints)
  const volumePointsRef = useRef(volumePoints)
  candlePointsRef.current = candlePoints
  volumePointsRef.current = volumePoints

  useEffect(() => {
    let chart: IChartApi | null = null
    let cleanupResize: (() => void) | undefined

    async function initChart() {
      if (!containerRef.current) return

      const lc = await import('lightweight-charts')
      const h = Math.max(320, containerRef.current.clientHeight)
      chart = lc.createChart(containerRef.current, {
        layout: {
          background: { color: 'transparent' },
          textColor: '#9ca3af',
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

      seriesRef.current = chart.addSeries(
        CandlestickSeries,
        {
          upColor: positive,
          downColor: negative,
          borderDownColor: negative,
          borderUpColor: positive,
          wickDownColor: negative,
          wickUpColor: positive,
        },
        0
      )
      seriesRef.current.setData(candlePointsRef.current)

      const volumePane = chart.addPane()
      volumePane.setHeight(volumePaneHeightPx(h))

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
      }

      const resizeObserver = new ResizeObserver(() => applySize())
      resizeObserver.observe(containerRef.current)
      cleanupResize = () => resizeObserver.disconnect()
    }

    void initChart()

    return () => {
      cleanupResize?.()
      if (chart) {
        chart.remove()
        chartRef.current = null
        seriesRef.current = null
        volumeSeriesRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (!seriesRef.current || !volumeSeriesRef.current) return
    seriesRef.current.setData(candlePoints)
    volumeSeriesRef.current.setData(volumePoints)
    chartRef.current?.timeScale().fitContent()
  }, [candlePoints, volumePoints])

  return (
    <div
      ref={containerRef}
      className="w-full min-h-[min(52vh,560px)] h-[min(52vh,560px)]"
      data-testid="price-chart-lightweight-canvas"
      aria-hidden
    />
  )
}
