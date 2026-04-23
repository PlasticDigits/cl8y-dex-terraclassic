import { useEffect, useRef } from 'react'
import { CandlestickSeries } from 'lightweight-charts'
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'

const CHART_HEIGHT_PX = 400

interface PriceChartLightweightCanvasProps {
  candlePoints: ChartCandlePoint[]
}

/** Mounts TradingView lightweight-charts only when there is at least one valid OHLC point. */
export function PriceChartLightweightCanvas({ candlePoints }: PriceChartLightweightCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const candlePointsRef = useRef(candlePoints)
  candlePointsRef.current = candlePoints

  useEffect(() => {
    let chart: IChartApi | null = null
    let cleanupResize: (() => void) | undefined

    async function initChart() {
      if (!containerRef.current) return

      const lc = await import('lightweight-charts')
      chart = lc.createChart(containerRef.current, {
        layout: {
          background: { color: 'transparent' },
          textColor: '#9ca3af',
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
        height: CHART_HEIGHT_PX,
      })

      const positive =
        getComputedStyle(document.documentElement).getPropertyValue('--color-positive').trim() || '#22c55e'
      const negative =
        getComputedStyle(document.documentElement).getPropertyValue('--color-negative').trim() || '#ef4444'
      seriesRef.current = chart.addSeries(CandlestickSeries, {
        upColor: positive,
        downColor: negative,
        borderDownColor: negative,
        borderUpColor: positive,
        wickDownColor: negative,
        wickUpColor: positive,
      })
      seriesRef.current.setData(candlePointsRef.current)
      chart.timeScale().fitContent()

      chartRef.current = chart

      const resizeObserver = new ResizeObserver(() => {
        if (containerRef.current && chart) {
          chart.applyOptions({ width: containerRef.current.clientWidth })
        }
      })
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
      }
    }
  }, [])

  useEffect(() => {
    if (!seriesRef.current) return
    seriesRef.current.setData(candlePoints)
    chartRef.current?.timeScale().fitContent()
  }, [candlePoints])

  return (
    <div ref={containerRef} className="w-full min-h-[400px]" data-testid="price-chart-lightweight-canvas" aria-hidden />
  )
}
