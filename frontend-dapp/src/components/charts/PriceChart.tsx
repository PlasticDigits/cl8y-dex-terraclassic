import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCandles } from '@/services/indexer/client'
import type { IndexerCandle } from '@/types'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import type { IChartApi, ISeriesApi } from 'lightweight-charts'

const INTERVALS = ['1m', '5m', '15m', '1h', '4h', '1d'] as const

interface PriceChartProps {
  pairAddress: string
  defaultInterval?: string
}

export default function PriceChart({ pairAddress, defaultInterval = '1h' }: PriceChartProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const [interval, setInterval_] = useState(defaultInterval)

  const candlesQuery = useQuery({
    queryKey: ['candles', pairAddress, interval],
    queryFn: () => getCandles(pairAddress, interval),
    refetchInterval: 30_000,
    enabled: !!pairAddress,
  })

  useEffect(() => {
    let chart: IChartApi | null = null

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
        height: 400,
      })

      const positive =
        getComputedStyle(document.documentElement).getPropertyValue('--color-positive').trim() || '#22c55e'
      const negative =
        getComputedStyle(document.documentElement).getPropertyValue('--color-negative').trim() || '#ef4444'
      seriesRef.current = chart.addCandlestickSeries({
        upColor: positive,
        downColor: negative,
        borderDownColor: negative,
        borderUpColor: positive,
        wickDownColor: negative,
        wickUpColor: positive,
      })

      chartRef.current = chart

      const resizeObserver = new ResizeObserver(() => {
        if (containerRef.current) {
          chart.applyOptions({ width: containerRef.current.clientWidth })
        }
      })
      resizeObserver.observe(containerRef.current)

      return () => resizeObserver.disconnect()
    }

    initChart()

    return () => {
      if (chart) {
        chart.remove()
        chartRef.current = null
        seriesRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (!seriesRef.current || !candlesQuery.data) return

    const data = candlesQuery.data
      .filter((c: IndexerCandle) => c.open && c.close)
      .map((c: IndexerCandle) => ({
        time: Math.floor(new Date(c.open_time).getTime() / 1000) as number,
        open: parseFloat(c.open),
        high: parseFloat(c.high),
        low: parseFloat(c.low),
        close: parseFloat(c.close),
      }))
      .sort((a, b) => (a.time as number) - (b.time as number))

    seriesRef.current.setData(data)
    chartRef.current?.timeScale().fitContent()
  }, [candlesQuery.data])

  return (
    <div className="shell-panel-strong">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
          Price Chart
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
        <div className="flex items-center justify-center h-[400px] gap-3" style={{ color: 'var(--ink-subtle)' }}>
          <Spinner /> Loading chart...
        </div>
      )}

      {candlesQuery.isError && (
        <div className="flex items-center justify-center h-[400px] text-red-400 text-sm uppercase tracking-wide font-semibold">
          Failed to load chart data
        </div>
      )}

      <div ref={containerRef} className={candlesQuery.isLoading || candlesQuery.isError ? 'hidden' : ''} />
    </div>
  )
}
